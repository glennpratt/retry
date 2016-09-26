pub mod exit_code_ext;

use exit_code_ext::ExitCodeExt;

use std::io;
use std::io::Write;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};
use std::thread::sleep;

//#[derive(Debug)]
pub struct RetryCommand {
    command: Command,
    retry_delay: Duration,
    retry_timeout: Duration,
    retry_until: Vec<i32>,
    retry_on: Option<Vec<i32>>,
    rewrite: Vec<(i32, i32)>,
    logger: Option<Box<Write>>
}

/// Builder for running a `Command` repeatedly until a desired state or timeout
/// is reached.
impl RetryCommand {
    /// Constructs a new `RetryCommand`. By default the command be run without
    /// retries. See `retry_timeout`.
    pub fn new(command: Command) -> RetryCommand {
        RetryCommand {
            command: command,
            retry_timeout: Duration::from_secs(0),
            retry_until: vec![0],
            retry_on: None,
            retry_delay: Duration::from_secs(0),
            rewrite: vec![],
            logger: None
        }
    }

    /// Adds a timeout. If a `Command`s exit code doesn't match a `retry_until`
    /// value and matches a `retry_on` value (if set), it will be retried until
    /// this `Duration` expires.
    pub fn retry_timeout(&mut self, value: Duration) -> &mut Self {
        self.retry_timeout = value;
        self
    }

    /// When a `Command` will be retried, sleep this `Duration` first.
    pub fn retry_delay(&mut self, value: Duration) -> &mut Self {
        self.retry_delay = value;
        self
    }

    /// Vec of exit codes which represent a desired exit code.
    /// Default to `[0]`.
    pub fn retry_until(&mut self, value: Vec<i32>) -> &mut Self {
        self.retry_until = value;
        self
    }

    /// `Vec` of exit codes which represent an exit code that may be retried.
    /// By default, all non-zero exit codes are retried.
    pub fn retry_on(&mut self, value: Vec<i32>) -> &mut Self {
        self.retry_on = Some(value);
        self
    }

    /// Rewrite the final exit code with a corresponding tuple value.
    pub fn rewrite(&mut self, value: Vec<(i32, i32)>) -> &mut Self {
        self.rewrite = value;
        self
    }

    /// When provided, log messages will be written to this object.
    pub fn logger(&mut self, value: Box<Write>) -> &mut Self {
        self.logger = Some(value);
        self
    }

    /// Run the command with retries and return shell-like exit code.
    pub fn exit_code(&mut self) -> io::Result<i32> {
        let (_, code) = try!(self.status_and_code());
        Ok(code)
    }

    /// Run the command with retries and return ExitStatus struct.
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        let (result, _) = try!(self.status_and_code());
        result
    }

    fn status_and_code(&mut self) -> io::Result<(io::Result<ExitStatus>, i32)> {
        let start = Instant::now();

        loop {
            let result = self.command.status();

            let (code, msg_opt) = try!(result.exit_code());

            if let Some(msg) = msg_opt {
                self.log(msg);
            }

            if self.should_stop(code, start) {
                return Ok((result, self.rewrite_code(code)));
            } else {
                sleep(self.retry_delay);
            }
        }
    }

    fn log(&mut self, msg: String) {
        if let Some(ref mut io) = self.logger {
            writeln!(io, "{:?} {}", self.command, msg).unwrap_or(());
        }
    }

    fn rewrite_code(&self, code: i32) -> i32 {
        // Only apply one rewrite, the last match, hence rev().
        for &(from_code, to_code) in self.rewrite.iter().rev() {
            if from_code == code {
                return to_code;
            }
        }
        return code;
    }

    fn should_stop(&self, code: i32, start: Instant) -> bool {
        if self.retry_until.contains(&code) {
            return true;
        }
        let ret = match self.retry_on {
            Some(ref retry_on) => !retry_on.contains(&code),
            None => false
        };
        (ret || ((Instant::now() - start) >= self.retry_timeout))
    }
}
