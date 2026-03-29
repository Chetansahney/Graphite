use crate::raster_types::{CPU, Raster};
use core_types::Ctx;
use graphite_ml_ipc::{BlockingIpcClient, InferenceRequest, InferenceStatus, IpcConfig};
use once_cell::sync::Lazy;
use serde_json::json;

static IPC_CLIENT: Lazy<BlockingIpcClient> = Lazy::new(|| BlockingIpcClient::new(IpcConfig::default()).expect("failed to initialize ML IPC client runtime"));

/// Experimental SAM2 placeholder that forwards a request to the ML IPC client.
#[node_macro::node(category("ML: Experimental"))]
pub fn sam2_mask_inference(_: impl Ctx, mask: Raster<CPU>, prompt: String) -> Raster<CPU> {
	let mut request = InferenceRequest::new("sam2", Vec::<u8>::new());
	let prompt_for_log = prompt.clone();
	request
		.parameters
		.insert("mask_metadata".to_string(), json!({ "width": mask.width, "height": mask.height, "prompt": prompt }));

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
