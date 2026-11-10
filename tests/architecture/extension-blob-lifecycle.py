"""Source wiring gate; physical cleanup and executor ownership have compiled regressions."""
from pathlib import Path
import argparse


def ordered(source: str, *tokens: str) -> None:
    position = 0
    for token in tokens:
        found = source.find(token, position)
        assert found >= 0, f"Missing ordered lifecycle boundary: {token}"
        position = found + len(token)


def check(root: Path) -> None:
    desktop = root / "rust/apps/nxtools-desktop/src"
    main = (desktop / "main.rs").read_text(encoding="utf-8")
    runtime = (desktop / "runtime.rs").read_text(encoding="utf-8")
    service = (desktop / "extensions/service.rs").read_text(encoding="utf-8")
    # The join is outside the completed GPUI callback, and failure cannot skip Clipboard flush.
    ordered(
        main,
        '.run(move |cx: &mut App| {',
        'cx.set_global(PlatformEventDispatcherGlobal::new(dispatcher));\n        });',
        'let extension_shutdown = runtime.extension_service.shutdown_and_wait();',
        'let clipboard_shutdown = worker_shutdown.shutdown();',
        'extension_shutdown.context(',
        'clipboard_shutdown.context(',
        'drop(runtime);',
    )
    headless = runtime.split("pub fn run_background_download_host(", 1)[1].split(
        "fn open_download_engine_from_config(", 1
    )[0]
    assert "let (extensions, _)" in headless
    ordered(
        headless,
        "activity.begin_draining();",
        ".shutdown_and_wait()",
        "activity.mark_stopped();",
        "return Ok(());",
    )
    ordered(
        service,
        'let worker = std::thread::Builder::new()',
        '.name("extension-runtime".into())',
        'let result = executor.block_on(async {',
        'let _ = stop.await;',
        'let result = runtime_shutdown.shutdown_and_wait().await;',
        'executor.shutdown_timeout(',
        'thread: Some(worker)',
    )
    ordered(service, "fn join(&self)", "self.stop();", "worker.thread.take()", "worker.result = Some(", ".join()", "worker\n            .result")
    assert 'self._lease.join()' in service
    print("PASS extension Blob exit wiring: GUI/headless explicit join, original live executor drain")
    print("Scope: source order only; compiled temporary-file and service tests prove lifecycle behavior.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    check(parser.parse_args().root)
