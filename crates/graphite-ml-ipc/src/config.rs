use std::{ffi::OsString, path::PathBuf, time::Duration};

pub const DEFAULT_READY_TOKEN: &str = "RUNNER_READY";

#[derive(Clone, Debug)]
pub struct SocketConfig {
	pub unix_path: PathBuf,
	pub windows_pipe: String,
}

impl Default for SocketConfig {
	fn default() -> Self {
		Self {
			unix_path: PathBuf::from("/tmp/graphite-ml-ipc.sock"),
			windows_pipe: String::from("graphite-ml-ipc"),
		}
	}
}

#[derive(Clone, Debug)]
pub struct RunnerConfig {
	pub command: OsString,
	pub args: Vec<OsString>,
	pub ready_token: String,
}

impl Default for RunnerConfig {
	fn default() -> Self {
		Self {
			command: OsString::from("graphite-ml-runner"),
			args: Vec::new(),
			ready_token: DEFAULT_READY_TOKEN.to_string(),
		}
	}
}

#[derive(Clone, Debug)]
pub struct IpcConfig {
	pub socket: SocketConfig,
	pub runner: RunnerConfig,
	pub idle_timeout: Duration,
	pub connect_timeout: Duration,
}

impl Default for IpcConfig {
	fn default() -> Self {
		Self {
			socket: SocketConfig::default(),
			runner: RunnerConfig::default(),
			idle_timeout: Duration::from_secs(60),
			connect_timeout: Duration::from_secs(5),
		}
	}
}

impl SocketConfig {
	#[cfg(unix)]
	pub fn unix_path(&self) -> &PathBuf {
		&self.unix_path
	}

	#[cfg(windows)]
	pub fn windows_pipe_name(&self) -> String {
		format!(r"\\.\pipe\{}", self.windows_pipe)
	}
}
