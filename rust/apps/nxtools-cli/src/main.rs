use std::process::ExitCode;

use nxtools_cli::{default_download_directory, parse_invocation, run_with_host_policy};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let archive_json = nxtools_cli::archive::is_archive_invocation(&arguments)
        && arguments.iter().any(|a| a == "--json");
    let invocation = match parse_invocation(arguments.clone(), default_download_directory()) {
        Ok(invocation) => invocation,
        Err(error) => {
            if archive_json {
                if let Err(output_error) = nxtools_cli::archive::emit_syntax_failure(&arguments) {
                    eprintln!("{}", output_error.message);
                }
            } else {
                eprintln!("{}", error.message);
            }
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
