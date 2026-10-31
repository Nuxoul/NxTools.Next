fn main() {
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        != [std::ffi::OsString::from("--archive-worker")]
    {
        std::process::exit(2);
    }
    if nxtools_archive::worker::run(std::io::stdin(), std::io::stdout()).is_err() {
        // Fixed exit category only. Never print private frames, paths, passwords or backend text.
        std::process::exit(1);
    }
}
