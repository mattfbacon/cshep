use anyhow::Context as _;
use std::ffi::{OsStr, OsString};
use std::process::{Command, Stdio};

pub struct RefCounter {
	pub start_command: OsString,
	pub end_command: OsString,
	pub check_success: bool,
	count: u64,
	should_exit: bool,
}

impl RefCounter {
	pub fn new(start_command: OsString, end_command: OsString, check_success: bool) -> Self {
		Self {
			start_command,
			end_command,
			check_success,
			count: u64::MIN,
			should_exit: false,
		}
	}
	pub fn count(&self) -> u64 {
		self.count
	}
}

impl RefCounter {
	pub fn increment(&mut self) -> anyhow::Result<()> {
		if self.count == u64::MIN {
			Self::run_command(&self.start_command, self.check_success).context("Running start command")?;
		}
		self.count = self.count.checked_add(1).ok_or_else(|| anyhow::anyhow!("Reference count overflowed"))?;
		Ok(())
	}
	pub fn decrement(&mut self) -> anyhow::Result<()> {
		self.count = self.count.checked_sub(1).ok_or_else(|| anyhow::anyhow!("Attempt to decrement already-zero reference count"))?;
		if self.count == u64::MIN {
			Self::run_command(&self.end_command, self.check_success).context("Running end command")?;
		}
		Ok(())
	}
	pub fn should_exit(&self) -> bool {
		self.should_exit
	}
	pub fn exit(&mut self) -> bool {
		if self.count == 0 {
			self.should_exit = true;
			true
		} else {
			false
		}
	}
	pub fn force_exit(&mut self) {
		self.should_exit = true;
	}
}

impl RefCounter {
	/// The command runs synchronously.
	/// This means that the thread that called `increment` or `decrement` will hold a write lock on the `RefCounter` while the process is executing.
	/// This is intentional and makes sure that commands run in a strict, total ordering between threads (and thus clients).
	fn run_command(command: &OsStr, check_success: bool) -> anyhow::Result<()> {
		let exit_status = Command::new("sh")
			.arg("-c")
			.arg(command)
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::inherit())
			.spawn()
			.context("Spawning command")?
			.wait()
			.context("Waiting for command")?;
		if check_success && !exit_status.success() {
			anyhow::bail!("Command failed");
		}
		Ok(())
	}
}

impl Drop for RefCounter {
	fn drop(&mut self) {
		if self.count > 0 {
			Self::run_command(&self.end_command, self.check_success).expect("Running end command at program termination")
		}
	}
}
