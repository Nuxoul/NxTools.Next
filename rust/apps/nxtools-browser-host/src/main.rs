use std::process::ExitCode;

fn main() -> ExitCode {
    match nxtools_browser_host::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("NxTools browser host stopped: {error:#}");
            ExitCode::FAILURE
        }
    }
}
