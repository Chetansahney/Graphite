mod blocking;
mod client;
mod config;
mod message;
mod process;
mod transport;

pub use blocking::BlockingIpcClient;
pub use client::AsyncIpcClient;
pub use config::{IpcConfig, RunnerConfig, SocketConfig};
pub use message::{InferenceRequest, InferenceResponse, InferenceStatus};

use std::time::Duration;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("serialization error: {0}")]
	Encode(#[from] rmp_serde::encode::Error),
	#[error("deserialization error: {0}")]
	Decode(#[from] rmp_serde::decode::Error),
	#[error("runner failed to signal readiness within {0:?}")]
	ReadyTimeout(Duration),
	#[error("runner output closed before ready signal")]
	RunnerNotReady,
	#[error("connection is not available")]
	Disconnected,
	#[error("response channel closed")]
	ResponseChannelClosed,
	#[error("runner spawn failed: {0}")]
	Spawn(String),
}
