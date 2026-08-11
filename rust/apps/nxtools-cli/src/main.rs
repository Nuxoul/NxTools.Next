use std::process::ExitCode;

use nxtools_cli::{default_download_directory, parse_invocation, run_with_host_policy};

fn main() -> ExitCode {
    let invocation = match parse_invocation(std::env::args().skip(1), default_download_directory())
    {
        Ok(invocation) => invocation,
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
    match run_with_host_policy(invocation.command, &executable, invocation.host_policy) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.message.is_empty() {
                eprintln!("{}", error.message);
            }
            ExitCode::from(error.exit_code)
        }
    }
}
