use crate::{Error, Result, config::RunnerConfig};
use std::{sync::Arc, time::Duration};
use tokio::{
	io::AsyncBufReadExt,
	process::{Child, Command},
	sync::Mutex,
	task::JoinHandle,
};

pub struct ProcessManager {
	runner: RunnerConfig,
	idle_timeout: Duration,
	connect_timeout: Duration,
	child: Arc<Mutex<Option<Child>>>,
	idle_task: Mutex<Option<JoinHandle<()>>>,
}

impl ProcessManager {
	pub fn new(runner: RunnerConfig, idle_timeout: Duration, connect_timeout: Duration) -> Self {
		Self {
			runner,
			idle_timeout,
			connect_timeout,
			child: Arc::new(Mutex::new(None)),
			idle_task: Mutex::new(None),
		}
	}

	pub async fn ensure_running(&self) -> Result<()> {
		let mut guard = self.child.lock().await;
		let needs_spawn = match guard.as_mut() {
			None => true,
			Some(child) => match child.try_wait() {
				Ok(Some(_)) => true,
				Ok(None) => false,
				Err(err) => return Err(Error::Spawn(err.to_string())),
			},
		};

		if needs_spawn {
			let mut child = self.spawn_child().await?;
			self.wait_for_ready(&mut child).await?;
			*guard = Some(child);
		}

		drop(guard);
		self.restart_idle_timer().await;
		Ok(())
	}

	pub async fn touch(&self) {
		self.restart_idle_timer().await;
	}

	async fn spawn_child(&self) -> Result<Child> {
		let mut command = Command::new(&self.runner.command);
		command.args(&self.runner.args);
		command.stdin(std::process::Stdio::null());
		command.stdout(std::process::Stdio::piped());
		command.stderr(std::process::Stdio::inherit());

		command.spawn().map_err(|err| Error::Spawn(err.to_string()))
	}

	async fn wait_for_ready(&self, child: &mut Child) -> Result<()> {
		let stdout = child.stdout.take().ok_or(Error::RunnerNotReady)?;
		let ready_token = self.runner.ready_token.clone();
		let connect_timeout = self.connect_timeout;
		let mut lines = tokio::io::BufReader::new(stdout).lines();

		let ready = tokio::time::timeout(connect_timeout, async {
			while let Some(line) = lines.next_line().await? {
				if line.trim() == ready_token {
					return Ok::<(), Error>(());
				}
			}
			Err(Error::RunnerNotReady)
		})
		.await;

		match ready {
			Ok(Ok(())) => {
				// Keep draining stdout to avoid blocking the runner.
				tokio::spawn(async move {
					while let Ok(Some(line)) = lines.next_line().await {
						log::trace!("runner: {line}");
					}
				});
				Ok(())
			}
			Ok(Err(err)) => Err(err),
			Err(_) => Err(Error::ReadyTimeout(connect_timeout)),
		}
	}

	async fn restart_idle_timer(&self) {
		if let Some(handle) = self.idle_task.lock().await.take() {
			handle.abort();
		}

		let timeout = self.idle_timeout;
		let child_slot = Arc::clone(&self.child);
		let handle = tokio::spawn(async move {
			tokio::time::sleep(timeout).await;
			let mut guard = child_slot.lock().await;
			if let Some(child) = guard.as_mut() {
				let _ = child.kill().await;
				let _ = child.wait().await;
			}
			*guard = None;
		});

		*self.idle_task.lock().await = Some(handle);
	}
}
