//! This module is called background because it launches a background process.
//!
//! These processes aren't truly daemons. The background process managed via
//! this file is only forked once, with no setsid call in between. This keeps
//! the fork in the same process group, even through the parent process still
//! exits, causing it to be reparented to init.
//!
//! In other words, this background process will get certain signals linked to
//! the process group, most notably our good friend ctrl-c.

use std::{
	fs,
	io::{self, Read},
	os::unix::process::CommandExt,
	path::{Path, PathBuf},
	process::{self, exit, Command},
};

use anyhow::{bail, Context, Result};
use log::{debug, info};

use crate::config::Config;

fn open_pid_file(
	config: &Config,
	create: bool,
) -> Result<Option<(fs::File, PathBuf, Option<i32>)>> {
	let angular_root_folder = &config.angular_root_folder;

	if !angular_root_folder.exists() {
		if !create {
			return Ok(None);
		}

		fs::create_dir_all(angular_root_folder)?;
	}

	let pid_file_path = angular_root_folder.join(".angular-cli.pid");
	let mut pid_file;
	let pid: Option<i32>;

	if pid_file_path.exists() {
		pid_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(create)
			.open(&pid_file_path)?;

		let mut pid_str = String::new();
		pid_file.read_to_string(&mut pid_str)?;

		let pid_ = pid_str.trim().parse::<i32>()?;
		debug!("Found PID {pid_}");

		if unsafe { libc::kill(pid_, 0) == libc::EXIT_SUCCESS } {
			pid = Some(pid_);
		} else {
			match io::Error::last_os_error().raw_os_error().unwrap() {
				libc::ESRCH => {
					pid = None;
				}
				_ => {
					bail!("Failed to send signal to Angular process");
				}
			}
		}
	} else {
		if !create {
			return Ok(None);
		}

		pid = None;
		pid_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(&pid_file_path)?;
	}

	Ok(Some((pid_file, pid_file_path, pid)))
}

pub(crate) fn stop(config: &Config) -> Result<()> {
	let Some((_, pid_file_path, pid)) = open_pid_file(config, false)? else {
		info!("No running angular process was found");
		return Ok(());
	};

	let Some(pid) = pid else {
		info!("Angular was already stopped");
		remove_file(pid_file_path)?;
		return Ok(());
	};

	info!("Stopping Angular process (PID {pid}) ...");

	if unsafe { libc::kill(pid, libc::SIGTERM) != libc::EXIT_SUCCESS } {
		match io::Error::last_os_error().raw_os_error().unwrap() {
			libc::ESRCH => {
				// ok, weird, angular has stopped in the time between sending the two
				// signals... whatever, let's be happy
				info!("Angular was already stopped");
				remove_file(pid_file_path)?;
				return Ok(());
			}
			_ => {
				bail!("Failed to send signal to Angular process");
			}
		}
	}

	info!("Sent termination signal");
	remove_file(pid_file_path)?;

	Ok(())
}

pub(super) fn is_running(config: &Config) -> Result<bool> {
	Ok(matches!(
		open_pid_file(config, false)?,
		Some((_, _, Some(_)))
	))
}

pub(super) fn start(config: &Config) -> Result<()> {
	let (pid_file, pid_file_path, pid) = open_pid_file(config, true)?.unwrap();

	if let Some(pid) = pid {
		bail!("Angular is already running at {pid}");
	}

	drop(pid_file);

	let pid = unsafe { libc::fork() };
	if pid < 0 {
		return Err(io::Error::last_os_error()).context("Failed to fork()");
	}

	if pid != 0 {
		// parent process
		return Ok(());
	}

	// inside the fork!

	unsafe {
		let devnull_fd = libc::open((b"/dev/null\0" as *const [u8; 10]).cast(), libc::O_RDWR);
		if devnull_fd == -1 {
			return Err(io::Error::last_os_error()).context("Failed to open /dev/null");
		}

		if libc::dup2(devnull_fd, libc::STDIN_FILENO) == -1 {
			return Err(io::Error::last_os_error()).context("Failed to close stdin");
		}

		if libc::close(devnull_fd) == -1 {
			return Err(io::Error::last_os_error()).context("Failed to close /dev/null");
		}
	}

	fs::write(pid_file_path, format!("{}", process::id()))?;

	let err = Command::new("ng")
		.arg("build")
		.arg("application")
		.arg("--watch")
		.current_dir(&config.angular_root_folder)
		.exec();

	log::error!("Failed to exec angular: {}", err);

	#[allow(clippy::exit)]
	exit(1);
}

/// Remove a file, but don't fail if it has already been removed
fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
	fs::remove_file(path).or_else(|err| match err.raw_os_error() {
		Some(libc::ENOENT) => Ok(()),
		_ => Err(err),
	})?;

	Ok(())
}
