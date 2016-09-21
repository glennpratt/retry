use std::error::Error;
use std::io;
use std::io::ErrorKind;
use std::process::{ExitStatus, Output};
use std::os::unix::process::ExitStatusExt;

/// Extension provides `exit_code()` method.
pub trait ExitCodeExt {
    /// Resolves the receiver into an exit code and an optional message if at
    /// all possible (including OS errors). Based on shell exit codes:
    ///
    /// http://tldp.org/LDP/abs/html/exitcodes.html
    ///
    /// # Examples
    ///
    /// ```
    /// use retry_command::exit_code_ext::ExitCodeExt;
    ///
    /// let result = std::process::Command::new("false").status();
    /// let (code, _) = result.exit_code().unwrap();
    /// assert_eq!(1, code);
    ///
    /// let result = std::process::Command::new("/dev/null").status();
    /// let (code, msg_opt) = result.exit_code().unwrap();
    /// assert_eq!(126, code);
    /// let expected_msg = "Permission denied (os error 13)".to_string();
    /// assert_eq!(Some(expected_msg), msg_opt);
    /// ```
    fn exit_code(&self) -> io::Result<(i32, Option<String>)>;
}

impl ExitCodeExt for ExitStatus {
    fn exit_code(&self) -> io::Result<(i32, Option<String>)> {
        Ok(
            match self.code() {
                Some(exit_code) => (exit_code, None),
                None => match self.signal() {
                    Some(signal) => (signal + 128, None),
                    None => { return Err(
                        io::Error::new(ErrorKind::Other, "Unkonwn exit code")
                    )}
                }
            }
        )
    }
}

impl ExitCodeExt for io::Error {
    fn exit_code(&self) -> io::Result<(i32, Option<String>)> {
        Ok(
            (
                match self.kind() {
                    ErrorKind::NotFound => 127,
                    ErrorKind::PermissionDenied => 126,
                    _ => { return Err(
                        io::Error::new(self.kind(), self.description())
                    )}
                },
                Some(format!("{}", self))
            )
        )
    }
}

impl ExitCodeExt for io::Result<ExitStatus> {
    fn exit_code(&self) -> io::Result<(i32, Option<String>)> {
        Ok(
            match *self {
                Ok(ref status) => try!(status.exit_code()),
                Err(ref e) => try!(e.exit_code())
            }
        )
    }
}

impl ExitCodeExt for io::Result<Output> {
    fn exit_code(&self) -> io::Result<(i32, Option<String>)> {
        Ok(
            match *self {
                Ok(ref output) => try!(output.status.exit_code()),
                Err(ref e) => try!(e.exit_code())
            }
        )
    }
}
