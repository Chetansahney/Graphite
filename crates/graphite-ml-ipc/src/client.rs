use crate::{
	Error, Result,
	config::IpcConfig,
	message::{InferenceRequest, InferenceResponse, InferenceStatus, PROTOCOL_VERSION},
	process::ProcessManager,
	transport,
	transport::{TransportReader, TransportWriter},
};
use std::{collections::HashMap, sync::Arc};
use tokio::{
	sync::{Mutex, oneshot},
	task::JoinHandle,
};
use uuid::Uuid;

pub struct AsyncIpcClient {
	config: IpcConfig,
	process: ProcessManager,
	writer: Arc<Mutex<Option<TransportWriter>>>,
	reader_task: Mutex<Option<JoinHandle<()>>>,
	pending: Arc<Mutex<HashMap<Uuid, oneshot::Sender<InferenceResponse>>>>,
}

impl AsyncIpcClient {
	pub fn new(config: IpcConfig) -> Self {
		Self {
			process: ProcessManager::new(config.runner.clone(), config.idle_timeout, config.connect_timeout),
			config,
			writer: Arc::new(Mutex::new(None)),
			reader_task: Mutex::new(None),
			pending: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	pub async fn send(&self, request: InferenceRequest) -> Result<InferenceResponse> {
		self.process.ensure_running().await?;
		self.ensure_transport().await?;

		let bytes = rmp_serde::to_vec_named(&request)?;
		let (tx, rx) = oneshot::channel();
		self.pending.lock().await.insert(request.request_id, tx);

		{
			let mut writer_guard = self.writer.lock().await;
			let writer = writer_guard.as_mut().ok_or(Error::Disconnected)?;
			writer.send_frame(&bytes).await?;
		}

		self.process.touch().await;

		rx.await.map_err(|_| Error::ResponseChannelClosed)
	}

	pub async fn send_bytes(&self, model: impl Into<String>, payload: Vec<u8>) -> Result<InferenceResponse> {
		let request = InferenceRequest::new(model, payload);
		self.send(request).await
	}

	async fn ensure_transport(&self) -> Result<()> {
		let mut writer_guard = self.writer.lock().await;
		if writer_guard.is_some() {
			return Ok(());
		}

		let (reader, writer) = transport::connect(&self.config.socket, self.config.connect_timeout).await?;
		*writer_guard = Some(writer);
		self.spawn_reader(reader).await;

		Ok(())
	}

	async fn spawn_reader(&self, reader: TransportReader) {
		if let Some(handle) = self.reader_task.lock().await.take() {
			handle.abort();
		}

		let pending = Arc::clone(&self.pending);
		let writer_slot = Arc::clone(&self.writer);
		let handle = tokio::spawn(async move {
			let mut reader = reader;
			loop {
				match reader.read_frame().await {
					Ok(Some(bytes)) => match rmp_serde::from_slice::<InferenceResponse>(&bytes) {
						Ok(response) => {
							let sender = pending.lock().await.remove(&response.request_id);
							if let Some(sender) = sender {
								let _ = sender.send(response);
							} else {
								log::warn!("dropping unmatched response {}", response.request_id);
							}
						}
						Err(err) => {
							log::warn!("failed to decode inference response: {err}");
						}
					},
					Ok(None) => {
						log::info!("ipc transport closed");
						break;
					}
					Err(err) => {
						log::warn!("ipc transport read failed: {err}");
						break;
					}
				}
			}

			let mut writer_guard = writer_slot.lock().await;
			*writer_guard = None;

			let mut pending = pending.lock().await;
			for (request_id, sender) in pending.drain() {
				let _ = sender.send(InferenceResponse {
					request_id,
					status: InferenceStatus::Failed,
					payload: None,
					error: Some("connection closed".into()),
					version: PROTOCOL_VERSION,
				});
			}
		});

		*self.reader_task.lock().await = Some(handle);
	}
}
