use std::error::Error;
use std::env;
use std::fmt;
use std::io;

use getopts;

/// Exit code representing failures internal to retry, not the spawned process.
pub static ERR_EXIT_STATUS: i32 = 125;

/// Result meant to be returned from CLI commands.
pub type CliResult = Result<i32, CliError>;

/// Wraps all `Error`s that are allowed to bubble up through the CLI
/// application.
#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    Arguments(getopts::Fail),
    InvalidArgumentValue(String)
}

impl CliError {
    /// Returns the exit code for this exceptions.
    /// Currently, these exceptions all imply an internal failure that couldn't
    /// be resolved to a failure in the spawned command, so we use and exit
    /// somewhat distinct exit code.
    pub fn exit_code(&self) -> i32 {
        ERR_EXIT_STATUS
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let program = env::args().next().unwrap_or("retry".to_string());
        match *self {
            CliError::Io(ref err) => err.fmt(f),
            CliError::Arguments(ref err) => {
                write!(f, "{}\nTry '{} --help'", err, program)
            },
            CliError::InvalidArgumentValue(ref value) => write!(f, "{} isn't a valid value", value)
        }
    }
}

impl Error for CliError {
    fn description(&self) -> &str {
        match *self {
            CliError::Io(ref err) => err.description(),
            CliError::Arguments(ref err) => err.description(),
            CliError::InvalidArgumentValue(_) => "Invalid argument value"
        }
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError::Io(err)
    }
}

impl From<getopts::Fail> for CliError {
    fn from(err: getopts::Fail) -> CliError {
        CliError::Arguments(err)
    }
}
