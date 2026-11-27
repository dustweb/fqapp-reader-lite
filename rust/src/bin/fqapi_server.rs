use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use fqapi_core::service::{build_server, serve, ServiceOptions};

const USAGE: &str = "usage: fqapi_server --config PATH --pool PATH --pool-key-file PATH \
                     [--filter PATH] [--listen IP:PORT] [--mock-upstream URL]";

#[tokio::main]
async fn main() -> ExitCode {
    let options = match parse_args(std::env::args_os().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("fqapi_server: {error}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let requested = options.listen;
    if !requested.ip().is_loopback() {
        eprintln!("fqapi_server: --listen must be a loopback address");
        return ExitCode::FAILURE;
    }
    let server = match build_server(&options).await {
        Ok(server) => server,
        Err(error) => {
            eprintln!("fqapi_server: {error}");
            return ExitCode::FAILURE;
        }
    };

    let listener = match tokio::net::TcpListener::bind(requested).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("fqapi_server: listen {requested}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let actual = listener.local_addr().unwrap_or(requested);
    eprintln!("fqapi_server listening on {actual}");

    match serve(listener, server).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("fqapi_server: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args<I>(args: I) -> Result<ServiceOptions, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut config_path = None;
    let mut pool_path = None;
    let mut filter_path = None;
    let mut pool_key_path = None;
    let mut listen = None;
    let mut mock_upstream = None;

    while let Some(argument) = args.next() {
        let argument = argument
            .to_str()
            .ok_or_else(|| "argument is not valid UTF-8".to_string())?;
        let (name, inline) = match argument.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (argument, None),
        };

        match name {
            "--config" => {
                config_path = Some(PathBuf::from(next_value(&mut args, inline, name)?));
            }
            "--pool" => {
                pool_path = Some(PathBuf::from(next_value(&mut args, inline, name)?));
            }
            "--pool-key-file" => {
                pool_key_path = Some(PathBuf::from(next_value(&mut args, inline, name)?));
            }
            "--filter" => {
                filter_path = Some(PathBuf::from(next_value(&mut args, inline, name)?));
            }
            "--listen" => {
                let value = next_value(&mut args, inline, name)?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--listen is not valid UTF-8".to_string())?;
                listen = Some(
                    value
                        .parse::<SocketAddr>()
                        .map_err(|_| format!("invalid --listen address: {value}"))?,
                );
            }
            "--mock-upstream" => {
                let value = next_value(&mut args, inline, name)?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--mock-upstream URL is not valid UTF-8".to_string())?;
                mock_upstream = parse_mock_upstream(value)?;
            }
            _ => return Err(format!("unknown argument: {name}")),
        }
    }

    let required = |value: Option<PathBuf>, flag: &str| {
        value
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| format!("missing required {flag}"))
    };

    Ok(ServiceOptions {
        config_path: required(config_path, "--config PATH")?,
        pool_path: required(pool_path, "--pool PATH")?,
        filter_path: filter_path.unwrap_or_default(),
        pool_key_path: required(pool_key_path, "--pool-key-file PATH")?,
        listen: listen.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 18081))),
        mock_upstream,
    })
}

fn next_value<I>(args: &mut I, inline: Option<&str>, flag: &str) -> Result<OsString, String>
where
    I: Iterator<Item = OsString>,
{
    match inline {
        Some(value) => Ok(OsString::from(value)),
        None => args
            .next()
            .ok_or_else(|| format!("{flag} requires a value")),
    }
}

fn parse_mock_upstream(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed =
        url::Url::parse(value).map_err(|_| format!("invalid --mock-upstream URL: {value}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("--mock-upstream must use http or https".to_string());
    }
    Ok(Some(value.to_string()))
}
