use std::process::ExitCode;

use nxtools_cli::{browser_native_host, default_download_directory, parse_args, run};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if browser_native_host::is_native_messaging_invocation(&arguments) {
        return match browser_native_host::run_stdio() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!(
                    "{{\"event\":\"browser_native_host.stopped\",\"status\":\"failed\",\"error_code\":\"host_loop\",\"message\":{}}}",
                    serde_json::to_string(&error.to_string())
                        .unwrap_or_else(|_| "\"native host failed\"".into())
                );
                ExitCode::from(nxtools_cli::EXIT_OPERATION_FAILED)
            }
        };
    }
    let command = match parse_args(arguments, default_download_directory()) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::from(error.exit_code);
        }
    };
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("无法确定 CLI 位置：{error}");
            return ExitCode::from(nxtools_cli::EXIT_HOST_UNAVAILABLE);
        }
    };
    match run(command, &executable) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.message.is_empty() {
                eprintln!("{}", error.message);
            }
            ExitCode::from(error.exit_code)
        }
    }
}
