use crate::raster_types::{CPU, Raster};
use core_types::Ctx;
use graphite_ml_ipc::{BlockingIpcClient, InferenceRequest, InferenceStatus, IpcConfig};
use serde_json::json;
use std::sync::LazyLock;

fn build_ipc_client() -> BlockingIpcClient {
	BlockingIpcClient::new(IpcConfig::default()).unwrap_or_else(|err| panic!("failed to initialize ML IPC client runtime: {err}"))
}

static IPC_CLIENT: LazyLock<BlockingIpcClient> = LazyLock::new(build_ipc_client);

/// Experimental SAM2 placeholder that forwards a request to the ML IPC client.
#[node_macro::node(category("ML: Experimental"))]
pub fn sam2_mask_inference(_: impl Ctx, mask: Raster<CPU>, prompt: String) -> Raster<CPU> {
	let mut request = InferenceRequest::new("sam2", Vec::<u8>::new());
	let prompt_for_log = prompt.clone();
	request
		.parameters
		.insert("sam2_metadata".to_string(), json!({ "width": mask.width, "height": mask.height, "prompt": prompt }));

	match IPC_CLIENT.send(request) {
		Ok(response) => {
			if response.status == InferenceStatus::Completed {
				log::debug!("SAM2 inference completed for prompt `{prompt_for_log}`");
			} else {
				log::warn!("SAM2 inference response: {:?}", response.status);
			}
		}
		Err(err) => {
			log::warn!("SAM2 IPC request failed: {err}");
		}
	}

	// Placeholder: return the input mask until the runner emits actual mask data.
	mask
}
