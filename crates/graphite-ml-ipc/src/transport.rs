use crate::{Error, Result, config::SocketConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const FRAME_PREFIX_BYTES: usize = 4;
// Cap individual frames to 16 MiB to bound allocations and protect against malformed peers.
pub const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;

#[cfg(unix)]
type ReadHalf = tokio::net::unix::OwnedReadHalf;
#[cfg(unix)]
type WriteHalf = tokio::net::unix::OwnedWriteHalf;

#[cfg(windows)]
type ReadHalf = tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>;
#[cfg(windows)]
type WriteHalf = tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>;

pub enum TransportReader {
	#[cfg(unix)]
	Unix(ReadHalf),
	#[cfg(windows)]
	Windows(ReadHalf),
}

pub enum TransportWriter {
	#[cfg(unix)]
	Unix(WriteHalf),
	#[cfg(windows)]
	Windows(WriteHalf),
}

pub async fn connect(socket: &SocketConfig, timeout: Duration) -> Result<(TransportReader, TransportWriter)> {
	#[cfg(unix)]
	{
		use tokio::net::UnixStream;

		let stream = tokio::time::timeout(timeout, UnixStream::connect(&socket.unix_path()))
			.await
			.map_err(|_| Error::ReadyTimeout(timeout))??;
		let (reader, writer) = stream.into_split();
		return Ok((TransportReader::Unix(reader), TransportWriter::Unix(writer)));
	}

	#[cfg(windows)]
	{
		use tokio::io::split;
		use tokio::net::windows::named_pipe::ClientOptions;

		let path = socket.windows_pipe_name();
		let client = tokio::time::timeout(timeout, async move { ClientOptions::new().open(&path) })
			.await
			.map_err(|_| Error::ReadyTimeout(timeout))??;
		let (reader, writer) = split(client);
		return Ok((TransportReader::Windows(reader), TransportWriter::Windows(writer)));
	}
}

impl TransportWriter {
	pub async fn send_frame(&mut self, payload: &[u8]) -> Result<()> {
		let len = payload.len() as u32;
		match self {
			#[cfg(unix)]
			Self::Unix(writer) => {
				writer.write_u32(len).await?;
				writer.write_all(payload).await?;
				writer.flush().await?;
			}
			#[cfg(windows)]
			Self::Windows(writer) => {
				writer.write_u32(len).await?;
				writer.write_all(payload).await?;
				writer.flush().await?;
			}
		}
		Ok(())
	}
}

impl TransportReader {
	pub async fn read_frame(&mut self) -> Result<Option<Vec<u8>>> {
		let length = match self.read_length().await {
			Ok(Some(len)) => len,
			Ok(None) => return Ok(None),
			Err(err) => return Err(err),
		};

		let mut buffer = vec![0u8; length as usize];
		match self {
			#[cfg(unix)]
			Self::Unix(reader) => {
				if let Err(err) = reader.read_exact(&mut buffer).await {
					return if err.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(err.into()) };
				}
			}
			#[cfg(windows)]
			Self::Windows(reader) => {
				if let Err(err) = reader.read_exact(&mut buffer).await {
					return if err.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(err.into()) };
				}
			}
		}

		Ok(Some(buffer))
	}

	async fn read_length(&mut self) -> Result<Option<u32>> {
		let mut len_buf = [0u8; FRAME_PREFIX_BYTES];
		match self {
			#[cfg(unix)]
			Self::Unix(reader) => {
				if let Err(err) = reader.read_exact(&mut len_buf).await {
					return if err.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(err.into()) };
				}
			}
			#[cfg(windows)]
			Self::Windows(reader) => {
				if let Err(err) = reader.read_exact(&mut len_buf).await {
					return if err.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(err.into()) };
				}
			}
		}
		let length = u32::from_be_bytes(len_buf);
		if length > MAX_FRAME_BYTES {
			return Err(Error::FrameTooLarge(length));
		}
		Ok(Some(length))
	}
}
