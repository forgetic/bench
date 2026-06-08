use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use bench_forgejo::{ForgejoRunner, ForgejoServer};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

const USAGE: &str = "Usage:\n  bench forgejo download [--cache-dir <dir>]\n  bench forgejo up [--cache-dir <dir>] [--gomaxprocs <n>] [--with-runner]\n  bench forgejo serve [--cache-dir <dir>] [--gomaxprocs <n>] [--with-runner]\n\nOptions:\n  -h, --help              Print this help.\n  --cache-dir <dir>       Set BENCH_FORGEJO_CACHE_DIR before using bench-forgejo.\n  --gomaxprocs <n>        Set BENCH_FORGEJO_GOMAXPROCS before starting Forgejo.\n  --with-runner           Register and run a host-mode forgejo-runner.\n\nEnvironment:\n  BENCH_FORGEJO_CACHE_DIR      Binary cache directory.\n  BENCH_FORGEJO_GOMAXPROCS     GOMAXPROCS for spawned Forgejo processes.\n\nDeprecated aliases TEMPER_FORGEJO_CACHE_DIR and TEMPER_FORGEJO_GOMAXPROCS remain supported by the library.\n";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Forgejo(ForgejoCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ForgejoCommand {
    Download(DownloadOptions),
    Up(UpOptions),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DownloadOptions {
    cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct UpOptions {
    cache_dir: Option<PathBuf>,
    gomaxprocs: Option<String>,
    with_runner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UsageError(String);

fn main() {
    match parse_args(std::env::args_os().skip(1)) {
        Ok(Command::Help) => {
            print_usage_stdout();
        }
        Ok(Command::Forgejo(command)) => {
            if let Err(err) = execute(command) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("error: {}", err.0);
            print_usage_stderr();
            std::process::exit(2);
        }
    }
}

fn parse_args<I, S>(args: I) -> Result<Command, UsageError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    if args.is_empty() || args == [OsString::from("-h")] || args == [OsString::from("--help")] {
        return Ok(Command::Help);
    }

    match args.first().and_then(|arg| arg.to_str()) {
        Some("forgejo") => parse_forgejo(&args[1..]).map(Command::Forgejo),
        Some(other) => Err(UsageError(format!("unknown command `{other}`"))),
        None => Err(UsageError("arguments must be valid UTF-8".to_string())),
    }
}

fn parse_forgejo(args: &[OsString]) -> Result<ForgejoCommand, UsageError> {
    let action = args
        .first()
        .and_then(|arg| arg.to_str())
        .ok_or_else(|| UsageError("missing forgejo action".to_string()))?;
    match action {
        "download" => parse_download(&args[1..]).map(ForgejoCommand::Download),
        "up" | "serve" => parse_up(&args[1..]).map(ForgejoCommand::Up),
        other => Err(UsageError(format!("unknown forgejo action `{other}`"))),
    }
}

fn parse_download(args: &[OsString]) -> Result<DownloadOptions, UsageError> {
    let mut options = DownloadOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].to_str() {
            Some("--cache-dir") => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| UsageError("missing value for --cache-dir".to_string()))?;
                options.cache_dir = Some(PathBuf::from(value));
                i += 2;
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(UsageError(format!("unknown flag `{flag}`")));
            }
            Some(value) => return Err(UsageError(format!("unexpected argument `{value}`"))),
            None => return Err(UsageError("arguments must be valid UTF-8".to_string())),
        }
    }
    Ok(options)
}

fn parse_up(args: &[OsString]) -> Result<UpOptions, UsageError> {
    let mut options = UpOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].to_str() {
            Some("--cache-dir") => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| UsageError("missing value for --cache-dir".to_string()))?;
                options.cache_dir = Some(PathBuf::from(value));
                i += 2;
            }
            Some("--gomaxprocs") => {
                let value = args
                    .get(i + 1)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| UsageError("missing value for --gomaxprocs".to_string()))?;
                options.gomaxprocs = Some(value.to_string());
                i += 2;
            }
            Some("--with-runner") => {
                options.with_runner = true;
                i += 1;
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(UsageError(format!("unknown flag `{flag}`")));
            }
            Some(value) => return Err(UsageError(format!("unexpected argument `{value}`"))),
            None => return Err(UsageError("arguments must be valid UTF-8".to_string())),
        }
    }
    Ok(options)
}

fn execute(command: ForgejoCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        ForgejoCommand::Download(options) => run_download(options),
        ForgejoCommand::Up(options) => run_up(options),
    }
}

fn run_download(options: DownloadOptions) -> Result<(), Box<dyn std::error::Error>> {
    apply_cache_dir(options.cache_dir);
    let forgejo = bench_forgejo::download::ensure_binary()?;
    let runner = bench_forgejo::download::ensure_runner_binary()?;
    println!("forgejo={}", forgejo.display());
    println!("forgejo_runner={}", runner.display());
    Ok(())
}

fn run_up(options: UpOptions) -> Result<(), Box<dyn std::error::Error>> {
    apply_cache_dir(options.cache_dir);
    if let Some(value) = options.gomaxprocs {
        std::env::set_var("BENCH_FORGEJO_GOMAXPROCS", value);
    }

    let server = ForgejoServer::start()?;
    let runner = if options.with_runner {
        Some(ForgejoRunner::register(&server)?)
    } else {
        None
    };

    println!("base_url={}", server.base_url());
    println!("config_path={}", server.config_path().display());
    println!("data_dir={}", server.data_dir().display());
    if let Some(runner) = &runner {
        println!("runner_name={}", runner.name());
    }
    std::io::stdout().flush()?;

    wait_for_shutdown_signal()?;
    drop(runner);
    drop(server);
    Ok(())
}

fn apply_cache_dir(cache_dir: Option<PathBuf>) {
    if let Some(path) = cache_dir {
        std::env::set_var("BENCH_FORGEJO_CACHE_DIR", path);
    }
}

fn wait_for_shutdown_signal() -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    if signals.forever().next().is_some() {
        return Ok(());
    }
    Ok(())
}

fn print_usage_stdout() {
    println!("{USAGE}");
}

fn print_usage_stderr() {
    eprintln!("{USAGE}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, UsageError> {
        parse_args(args.iter().copied())
    }

    #[test]
    fn download_routes_to_download_command() {
        assert_eq!(
            parse(&["forgejo", "download"]).unwrap(),
            Command::Forgejo(ForgejoCommand::Download(DownloadOptions::default()))
        );
    }

    #[test]
    fn download_parses_cache_dir() {
        assert_eq!(
            parse(&["forgejo", "download", "--cache-dir", "cache"]).unwrap(),
            Command::Forgejo(ForgejoCommand::Download(DownloadOptions {
                cache_dir: Some(PathBuf::from("cache")),
            }))
        );
    }

    #[test]
    fn up_parses_up_command() {
        assert_eq!(
            parse(&["forgejo", "up"]).unwrap(),
            Command::Forgejo(ForgejoCommand::Up(UpOptions::default()))
        );
    }

    #[test]
    fn serve_parses_as_up_command() {
        assert_eq!(
            parse(&["forgejo", "serve"]).unwrap(),
            parse(&["forgejo", "up"]).unwrap()
        );
    }

    #[test]
    fn up_parses_runner_enablement() {
        assert_eq!(
            parse(&["forgejo", "up", "--with-runner"]).unwrap(),
            Command::Forgejo(ForgejoCommand::Up(UpOptions {
                with_runner: true,
                ..UpOptions::default()
            }))
        );
    }

    #[test]
    fn up_parses_cache_dir_and_gomaxprocs() {
        assert_eq!(
            parse(&["forgejo", "up", "--cache-dir", "cache", "--gomaxprocs", "2",]).unwrap(),
            Command::Forgejo(ForgejoCommand::Up(UpOptions {
                cache_dir: Some(PathBuf::from("cache")),
                gomaxprocs: Some("2".to_string()),
                with_runner: false,
            }))
        );
    }

    #[test]
    fn missing_flag_values_return_usage_errors() {
        assert!(parse(&["forgejo", "download", "--cache-dir"]).is_err());
        assert!(parse(&["forgejo", "up", "--cache-dir"]).is_err());
        assert!(parse(&["forgejo", "up", "--gomaxprocs"]).is_err());
    }

    #[test]
    fn unknown_action_returns_usage_error() {
        assert!(parse(&["forgejo", "runner"]).is_err());
    }

    #[test]
    fn unknown_flag_returns_usage_error() {
        assert!(parse(&["forgejo", "up", "--bad"]).is_err());
        assert!(parse(&["forgejo", "download", "--bad"]).is_err());
    }

    #[test]
    fn no_args_and_help_are_help_command() {
        assert_eq!(parse(&[]).unwrap(), Command::Help);
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["-h"]).unwrap(), Command::Help);
    }
}
