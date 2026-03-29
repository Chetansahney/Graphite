use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::collections::HashMap;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
	pub request_id: Uuid,
	pub model: String,
	pub payload: ByteBuf,
	#[serde(default)]
	pub parameters: HashMap<String, serde_json::Value>,
	#[serde(default = "default_protocol_version")]
	pub version: u32,
}

impl InferenceRequest {
	pub fn new(model: impl Into<String>, payload: impl Into<ByteBuf>) -> Self {
		Self {
			request_id: Uuid::new_v4(),
			model: model.into(),
			payload: payload.into(),
			parameters: HashMap::new(),
			version: PROTOCOL_VERSION,
		}
	}

	pub fn with_parameters(model: impl Into<String>, payload: impl Into<ByteBuf>, parameters: HashMap<String, serde_json::Value>) -> Self {
		Self {
			request_id: Uuid::new_v4(),
			model: model.into(),
			payload: payload.into(),
			parameters,
			version: PROTOCOL_VERSION,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InferenceStatus {
	Acknowledged,
	Running,
	Completed,
	Failed,
	Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
	pub request_id: Uuid,
	pub status: InferenceStatus,
	#[serde(default)]
	pub payload: Option<ByteBuf>,
	#[serde(default)]
	pub error: Option<String>,
	#[serde(default = "default_protocol_version")]
	pub version: u32,
}

fn default_protocol_version() -> u32 {
	PROTOCOL_VERSION
}
