extern crate retry_command;
extern crate getopts;

use getopts::Fail::{ArgumentMissing};
use retry_command::RetryCommand;
use std::env;
use std::io::{Write, stderr};
use std::process::{Command, exit};
use std::str::FromStr;
use std::time::Duration;

pub mod cli_error;
use cli_error::{CliError, CliResult};
use cli_error::CliError::{InvalidArgumentValue};

static NAME: &'static str = "retry";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let exit_code = match run(env::args().collect()) {
        Ok(code) => code,
        Err(e) => {
            // Ignore STDERR IO errors.
            writeln!(&mut stderr(), "{}", e).unwrap_or(());
            e.exit_code()
        }
    };
    exit(exit_code)
}

fn run(args: Vec<String>) -> CliResult {
    let (opts, matches) = try!(parse_args(&args[1..]));

    if matches.opt_present("help") {
        usage_command(opts)
    } else if matches.opt_present("version") {
        version_command()
    } else if matches.free.len() < 1 {
        Err(ArgumentMissing("-- COMMAND".into()).into())
    } else {
        let mut retry_cmd = try!(from_opts(matches));
        Ok(try!(retry_cmd.exit_code()))
    }
}

fn usage_command(opts: getopts::Options) -> CliResult {
    let program = env::args().next().unwrap_or(NAME.to_string());
    print!(
"{} {}

Usage:
{} [OPTIONS] -- COMMAND [ARGS]...

{}", NAME, VERSION, program, &opts.usage(""
    ));
    Ok(0)
}

fn version_command() -> CliResult {
    println!("{} {}", NAME, VERSION);
    Ok(0)
}

fn parse_args(args: &[String]) -> Result<(getopts::Options, getopts::Matches), getopts::Fail> {
    let mut opts = getopts::Options::new();
    opts.optopt("", "retry-timeout", "retry up to timeout seconds, then exit \
                                      with status 127", "TIMEOUT");
    opts.optopt("", "retry-delay", "wait delay seconds between each retry", "DELAY");
    opts.optmulti("", "retry-until", "retry until the exit code is one of the listed values (default 0)", "EXITCODE");
    opts.optmulti("", "retry-on", "retry if the exit code is one of the listed values", "EXITCODE");
    opts.optmulti("", "rewrite", "if the final exit status is a, change it to b; this happens after --retry-on/until processing", "<A>=<B>");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("v", "version", "output version information and exit");
    let matches = try!(opts.parse(args));
    Ok((opts, matches))
}

fn from_opts(matches: getopts::Matches) -> Result<RetryCommand, CliError> {
    let mut command = Command::new(&matches.free[0]);
    command.args(&matches.free[1..]);

    let mut retry_cmd = RetryCommand::new(command);
    retry_cmd.logger(Box::new(stderr()));

    if let Some(retry_timeout) = matches.opt_str("retry-timeout") {
        retry_cmd.retry_timeout(Duration::from_secs(
            try!(parse(&retry_timeout))
        ));
    }

    if let Some(retry_delay) = matches.opt_str("retry-delay") {
        retry_cmd.retry_delay(Duration::from_secs(
            try!(parse(&retry_delay))
        ));
    }

    for retry_on in matches.opt_strs("retry-on") {
        retry_cmd.retry_on.push(try!(parse(&retry_on)));
    }

    // Conditional to overwrite default only when present.
    if matches.opt_present("retry-until") {
        retry_cmd.retry_until(try!(
            matches.opt_strs("retry-until").iter().map(|retry_until|
              parse(retry_until)
            ).collect()
        ));
    }

    retry_cmd.rewrite(try!(
        matches.opt_strs("rewrite").iter().map(parse_rewrite).collect()
    ));

    Ok(retry_cmd)
}

// Add context to generic option value parsing.
fn parse<F>(string: &str) -> Result<F, CliError> where F: FromStr {
    match string.parse::<F>() {
        Ok(new_f_type) => Ok(new_f_type),
        Err(_e) => Err(InvalidArgumentValue(string.to_string()))
    }
}

fn parse_rewrite(rewrite: &String) -> Result<(i32,i32), CliError> {
    let mut iter = rewrite.split('=');
    if let Some(part_1) = iter.next() {
        if let Some(part_2) = iter.next() {
            if let None = iter.next() {
                return Ok((try!(parse(part_1)), try!(parse(part_2))));
            }
        }
    }
    Err(InvalidArgumentValue(rewrite.to_string()))
}
