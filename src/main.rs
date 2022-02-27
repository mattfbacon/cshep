use anyhow::Context as _;
use clap::Parser;
use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

mod counter;
use counter::RefCounter;

#[derive(Parser)]
struct Args {
	/// Control socket to listen on
	#[clap(short = 'S', long)]
	socket: PathBuf,
	/// Resource allocation command
	#[clap(short, long = "start")]
	start_command: OsString,
	/// Resource deallocation command
	#[clap(short, long = "end")]
	end_command: OsString,
	/// Whether to check the commands' exit codes for failure
	#[clap(short, long, parse(try_from_str), default_value = "true")]
	check_success: bool,
}

fn main() -> anyhow::Result<()> {
	let args = Args::parse();
	if args.socket.exists() {
		std::fs::remove_file(&args.socket).context("Removing old control socket")?;
	}
	let listener = UnixListener::bind(&args.socket).context("Listening on control socket")?;
	listener.set_nonblocking(true).context("Making control socket non-blocking")?;
	let counter = Arc::new(RwLock::new(RefCounter::new(args.start_command, args.end_command.clone(), args.check_success)));
	loop {
		if counter.read().unwrap().should_exit() {
			break;
		}
		match listener.accept() {
			Ok((client_socket, _)) => {
				let client = Client {
					stream: client_socket,
					counter: counter.clone(),
				};
				std::thread::spawn(move || client.run());
			}
			Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(10)),
			Err(err) => return Err(anyhow::Error::from(err).context("Accepting client on control socket")),
		};
	}
	std::fs::remove_file(&args.socket).context("Cleaning up control socket")?;
	Ok(())
}

struct Client {
	stream: UnixStream,
	counter: Arc<RwLock<RefCounter>>,
}

impl Client {
	fn try_run(mut self) -> anyhow::Result<()> {
		loop {
			if self.counter.read().unwrap().should_exit() {
				break Ok(());
			}
			let mut buf = [0u8; 1];
			match self.stream.read_exact(&mut buf) {
				Ok(_) => (),
				Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break Ok(()),
				Err(err) => return Err(anyhow::Error::from(err).context("Reading from socket client")),
			}
			match char::from(buf[0]) {
				// Add to the reference count
				'+' => self.counter.write().unwrap().increment().context("Incrementing reference count")?,
				// Subtract from the reference count
				'-' => self.counter.write().unwrap().decrement().context("Decrementing reference count")?,
				// Get the current refcount as an ASCII number followed by a newline
				'c' => writeln!(self.stream, "{}", self.counter.read().unwrap().count()).context("Writing reference count to socket")?,
				// Quit (only works if refcount == 0)
				'q' => {
					if !self.counter.write().unwrap().exit() {
						eprintln!("Attempt to quit while references still exist.");
					}
				}
				// Force quit regardless of refcount
				'Q' => {
					self.counter.write().unwrap().force_exit();
				}
				// Ignore extraneous characters to make manual usage (e.g. with `nc -U`) more pleasant
				_ => (),
			}
		}
	}
	pub fn run(self) {
		self.try_run().expect("Error");
	}
}
