use crate::{AsyncIpcClient, Error, InferenceRequest, InferenceResponse, IpcConfig, Result};
use tokio::runtime::{Builder, Runtime};

pub struct BlockingIpcClient {
	runtime: Runtime,
	client: AsyncIpcClient,
}

impl BlockingIpcClient {
	pub fn new(config: IpcConfig) -> Result<Self> {
		let runtime = Builder::new_multi_thread().enable_all().build().map_err(Error::Io)?;
		let client = AsyncIpcClient::new(config);
		Ok(Self { runtime, client })
	}

	pub fn send(&self, request: InferenceRequest) -> Result<InferenceResponse> {
		self.runtime.block_on(self.client.send(request))
	}

	pub fn send_bytes(&self, model: impl Into<String>, payload: Vec<u8>) -> Result<InferenceResponse> {
		self.runtime.block_on(self.client.send_bytes(model, payload))
	}
}
