mod sfx;

fn main() {
    let started = std::time::Instant::now();
    if let Err(error) = sfx::run() {
        eprintln!(
            "stage=failed error={:?} native_code={:?} files={} directories={} bytes={} cleanup_failed={} cleanup_native_code={:?} elapsed_ms={}",
            error.kind,
            error.native,
            error.completed.files,
            error.completed.directories,
            error.completed.bytes,
            error.cleanup_failed,
            error.cleanup_native,
            started.elapsed().as_millis()
        );
        std::process::exit(1);
    }
    println!(
        "stage=finished elapsed_ms={}",
        started.elapsed().as_millis()
    );
}
