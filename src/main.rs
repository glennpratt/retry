extern crate getopts;

#[macro_use] extern crate custom_derive;
#[macro_use] extern crate derive_builder;

use std::io::{ErrorKind, Write};
use std::process::Command;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::os::unix::process::ExitStatusExt;

macro_rules! println_stderr(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

static NAME: &'static str = "retry";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static ERR_EXIT_STATUS: i32 = 125;

fn main() {
    match run(std::env::args().collect()) {
        Ok(_) => {},
        Err(code) => std::process::exit(code)
    }
}

fn usage_command(program: String, opts: getopts::Options) -> Result<(), i32> {
    print!(
"{} {}

Usage:
{} [OPTIONS] -- COMMAND [ARGS]...

{}", NAME, VERSION, program, &opts.usage(""
    ));
    Ok(())
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
    opts.optflag("V", "version", "output version information and exit");
    let matches = try!(opts.parse(args));
    Ok((opts, matches))
}

fn run(args: Vec<String>) -> Result<(), i32> {
    let program = args[0].clone();
    let (opts, matches) = match parse_args(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println_stderr!("{}", f);
            return Err(ERR_EXIT_STATUS);
        }
    };

    if matches.opt_present("help") {
        usage_command(program, opts)
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        Ok(())
    } else if matches.free.len() < 1 {
        println_stderr!("missing an argument");
        println_stderr!("for help, try '{0} --help'", program);
        Err(ERR_EXIT_STATUS)
    } else {
        // TODO Don't clone here, move command into stuct.
        let options = try!(Options::try_from_opts(matches.clone()));
        let mut command = Command::new(&matches.free[0]);
        command.args(&matches.free[1..]);
        retry(command , options)
    }
}

fn retry(mut command: Command, options: Options) -> Result<(), i32> {
    let start = Instant::now();
    loop {
        let status = command.status();

        let mut code = match status {
            Ok(result) => match result.code() {
                Some(exit_code) => exit_code,
                None => match result.signal() {
                    Some(signal) => signal + 128,
                    None => {
                        println_stderr!("{:?} exit code unavailable.", command);
                        ERR_EXIT_STATUS
                    }
                }
            },
            Err(e) => {
                println_stderr!("{:?} {}", command, e);
                match e.kind() {
                    ErrorKind::NotFound => 127,
                    ErrorKind::PermissionDenied => 126,
                    _ => ERR_EXIT_STATUS
                }
            }
        };

        if (options.retry_until.contains(&code)) ||
           (!options.retry_on.is_empty() && !options.retry_on.contains(&code)) ||
           ((Instant::now() - start) >= options.retry_timeout) {

            // Only apply one rewrite, the last matching opt.
            for &(from_code, to_code) in options.rewrite.iter().rev() {
               if from_code == code {
                   code = to_code;
                   break;
               }
            }

            return Err(code);
        } else {
            sleep(options.retry_delay);
        }
    }
}

custom_derive! {
    #[derive(Builder, Debug)]
    pub struct Options {
        pub retry_timeout: Duration,
        pub retry_until: Vec<i32>,
        pub retry_on: Vec<i32>,
        pub retry_delay: Duration,
        pub rewrite: Vec<(i32, i32)>
    }
}

impl Default for Options {
    fn default() -> Options {
        Options {
            retry_timeout: Duration::from_secs(0),
            retry_until: vec![0],
            retry_on: vec![], // None and .len() == 0 are equivalent.
            retry_delay: Duration::from_secs(0),
            rewrite: vec![]
        }
    }
}

impl Options {
    pub fn try_from_opts(matches: getopts::Matches) -> Result<Options, i32> {
        let mut options = Options::default();

        if let Some(retry_timeout) = matches.opt_str("retry-timeout") {
            options.retry_timeout = Duration::from_secs(
                try!(Options::parse::<u64>(&retry_timeout))
            );
        }

        if let Some(retry_delay) = matches.opt_str("retry-delay") {
            options.retry_delay = Duration::from_secs(
                try!(Options::parse::<u64>(&retry_delay))
            );
        }

        for retry_on in matches.opt_strs("retry-on") {
            options.retry_on.push(try!(Options::parse::<i32>(&retry_on)));
        }

        // Conditional to overwrite default only when present.
        if matches.opt_present("retry-until") {
            options.retry_until = try!(
                matches.opt_strs("retry-until").iter().map(|retry_until| {
                  Options::parse::<i32>(&retry_until)
                }).collect()
            );
        }

        options.rewrite = try!(
            matches.opt_strs("rewrite").iter().map(|rewrite| {
                Options::parse_rewrite(rewrite)
            }).collect()
        );

        Ok(options)
    }

    // Add context to generic option value parsing.
    fn parse<F>(string: &str) -> Result<F, i32> where F: FromStr {
        match string.parse::<F>() {
            Ok(new_f_type) => Ok(new_f_type),
            Err(_e) => {
                // Can't call e.description() here... why?
                println!("Invalid option value: {}", string);
                // This validation really belongs at the getopt level.
                Err(ERR_EXIT_STATUS) // TODO stop bubbling code from so low.
            }
        }
    }

    fn parse_rewrite(rewrite: &str) -> Result<(i32,i32), i32> {
        let mut iter = rewrite.split('=');
        if let Some(part_1) = iter.next() {
            if let Some(part_2) = iter.next() {
                if let None = iter.next() {
                    return Ok((
                        try!(Options::parse::<i32>(part_1)),
                        try!(Options::parse::<i32>(part_2))
                    ));
                }
            }
        }
        println!("Invalid rewrite: {}", rewrite);
        Err(ERR_EXIT_STATUS)
    }
}
