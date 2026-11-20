from __future__ import annotations

import json
import os
import re
import subprocess
import sqlite3
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "installer" / "uninstall-cleanup.ps1"
POWERSHELL = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32" / "WindowsPowerShell" / "v1.0" / "powershell.exe"


def write(path: Path, text: str = "owned") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path
def write_clipboard_db(root: Path, asset: Path) -> Path:
    database = root / "clipboard.db"
    if database.exists():
        database.unlink()
    connection = sqlite3.connect(database)
    try:
        connection.execute("CREATE TABLE clipboard_items (image_path TEXT NOT NULL, source_process_icon_path TEXT NOT NULL)")
        connection.execute("CREATE TABLE clipboard_item_formats (data_path TEXT NOT NULL)")
        connection.execute("INSERT INTO clipboard_items(image_path, source_process_icon_path) VALUES (?, '')", (str(asset),))
        connection.commit()
    finally:
        connection.close()
    return database




def fixture(root: Path) -> dict[str, Path]:
    paths = {
        "install": root / "install",
        "config": root / "roaming" / "NxTools" / "NxTools.Next" / "config",
        "data": root / "roaming" / "NxTools" / "NxTools.Next" / "data",
        "cache": root / "local" / "NxTools" / "NxTools.Next" / "cache",
        "logs": root / "local" / "NxTools" / "NxTools.Next" / "data" / "logs",
        "clipboard": root / "custom-clipboard",
        "archive": root / "custom-archive",
    }
    for path in paths.values():
        path.mkdir(parents=True, exist_ok=True)
    write(paths["install"] / "NxTools.exe")
    output_data = write(paths["data"] / "downloads" / "kept-default.bin", "download output")
    output_cache = write(paths["cache"] / "nested" / "kept-cache.bin", "download output")
    write(paths["config"] / "config.json", json.dumps({
        "clipboard": {"storage_directory": str(paths["clipboard"])},
        "archive": {"temp": {"base_directory": str(paths["archive"])}}
    }))
    write(paths["config"] / "config.json.lock")
    database = paths["data"] / "downloads.sqlite3"
    connection = sqlite3.connect(database)
    try:
        connection.execute("CREATE TABLE download_artifacts (path TEXT NOT NULL, kind TEXT NOT NULL, state TEXT NOT NULL)")
        connection.executemany(
            "INSERT INTO download_artifacts(path, kind, state) VALUES (?, 'final', 'committed')",
            [(str(output_data),), (str(output_cache),)],
        )
        connection.commit()
    finally:
        connection.close()
    for name in ("downloads.sqlite3-wal", "downloads.sqlite3-shm",
                 "quick-launch.db", "quick-launch.db-wal", "quick-launch.db-shm",
                 "clipboard.db", "clipboard.db-wal", "clipboard.db-shm"):
        write(paths["data"] / name)
    write(paths["data"] / "clipboard-images" / "capture.png")
    write(paths["data"] / "extensions-data" / "runtime.json")
    write(paths["data"] / "browser-blob-staging" / "pending.blob")
    write(paths["data"] / "tools" / "ffmpeg" / "ffmpeg.exe")
    write(paths["cache"] / "ordinary-cache.bin")
    write(paths["logs"] / "desktop.log")
    for name in ("clipboard.db", "clipboard.db-wal", "clipboard.db-shm"):
        write(paths["clipboard"] / name)
    write(paths["clipboard"] / "clipboard-images" / "custom.png")
    write(paths["clipboard"] / "unrelated-sentinel.txt", "keep")
    sessions = paths["archive"] / "NxTools.Next" / "archive" / "sessions"
    write(sessions / "session" / "owned.bin")
    write(paths["archive"] / "unrelated-sentinel.txt", "keep")
    record = {"Version": 1, "BaseDirectory": str(paths["archive"]),
              "Identity": {"Volume": 1, "Low": 2, "High": 3}}
    write(paths["cache"] / "archive" / "temp-roots" / "0.json", json.dumps(record))
    paths["output_data"] = output_data
    paths["output_cache"] = output_cache
    paths["sessions"] = sessions
    return paths


def effects(root: Path, fail_process: bool = False) -> Path:
    path = root / "fixture-effects.ps1"
    failure = "throw 'fixture process stop denied'" if fail_process else ""
    path.write_text(f"""
function Stop-NxToolsOwnedProcesses {{
    param([string]$InstallDirectory, [string]$CloseLibraryPath)
    if (!(Test-Path -LiteralPath (Join-Path $InstallDirectory 'NxTools.exe'))) {{ throw 'production entry did not receive fixture install root' }}
    Add-Content -LiteralPath $env:NXTOOLS_FIXTURE_EVENTS -Value 'processes'
    {failure}
}}
function Remove-NxToolsOwnedRegistrations {{
    param([string]$InstallDirectory, [string]$LocalDataRoot)
    Add-Content -LiteralPath $env:NXTOOLS_FIXTURE_EVENTS -Value 'registrations'
}}
function Get-NxToolsDirectoryIdentity {{ param([string]$Path) return '1:2:3' }}
""", encoding="utf-8")
    return path


def invoke(paths: dict[str, Path], operation: str, remove: bool, effect_path: Path) -> subprocess.CompletedProcess[str]:
    events = paths["install"].parent / "events.txt"
    env = os.environ.copy()
    env["NXTOOLS_FIXTURE_EVENTS"] = str(events)
    command = [
        str(POWERSHELL), "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
        "-File", str(SCRIPT), "-Operation", operation,
        "-InstallDirectory", str(paths["install"]),
        "-ConfigRoot", str(paths["config"]), "-DataRoot", str(paths["data"]),
        "-CacheRoot", str(paths["cache"]), "-LogRoot", str(paths["logs"]),
        "-CloseLibraryPath", str(ROOT / "installer" / "close-owned-processes.ps1"),
        "-FixtureRoot", str(paths["install"].parent), "-FixtureEffectsPath", str(effect_path),
    ]
    if remove:
        command.append("-RemovePersonalData")
    return subprocess.run(command, cwd=ROOT, env=env, text=True, errors="replace", capture_output=True, check=False)

def assert_success(result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise AssertionError(f"cleanup failed ({result.returncode})\nstdout={result.stdout}\nstderr={result.stderr}")


def scenario_remove(root: Path) -> None:
    paths = fixture(root)
    result = invoke(paths, "Uninstall", True, effects(root))
    assert_success(result)
    events = (root / "events.txt").read_text(encoding="utf-8").splitlines()
    assert events == ["processes", "registrations"], events
    assert paths["output_data"].read_text(encoding="utf-8") == "download output"
    assert paths["output_cache"].read_text(encoding="utf-8") == "download output"
    assert not (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["data"] / "extensions-data" / "runtime.json").read_text(encoding="utf-8") == "owned"
    assert not (paths["clipboard"] / "clipboard.db").exists()
    assert (paths["clipboard"] / "clipboard-images" / "custom.png").read_text(encoding="utf-8") == "owned"
    assert (paths["clipboard"] / "unrelated-sentinel.txt").read_text(encoding="utf-8") == "keep"
    assert (paths["sessions"] / "session" / "owned.bin").read_text(encoding="utf-8") == "owned"
    assert (paths["archive"] / "unrelated-sentinel.txt").read_text(encoding="utf-8") == "keep"
    assert not (paths["config"] / "config.json").exists()
    assert (paths["cache"] / "ordinary-cache.bin").read_text(encoding="utf-8") == "owned"


def scenario_retain(root: Path) -> None:
    paths = fixture(root)
    result = invoke(paths, "Uninstall", False, effects(root))
    assert_success(result)
    assert (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["clipboard"] / "clipboard.db").exists()
    assert paths["sessions"].exists()
    assert (root / "events.txt").read_text(encoding="utf-8").splitlines() == ["processes", "registrations"]


def scenario_removed_history_configured_roots(root: Path) -> None:
    paths = fixture(root)
    configured_data_output = write(paths["data"] / "history-removed.bin", "user download")
    configured_category = paths["cache"] / "category-downloads"
    configured_category_output = write(configured_category / "category.bin", "user download")
    write(paths["config"] / "config.json", json.dumps({
        "download": {
            "default_directory": str(paths["data"]),
            "categories": [{"id": "archives", "name": "Archives", "enabled": True,
                            "save_directory": str(configured_category), "extensions": [], "url_patterns": []}],
        },
        "clipboard": {"storage_directory": str(paths["clipboard"])},
        "archive": {"temp": {"base_directory": str(paths["archive"])}}
    }))
    connection = sqlite3.connect(paths["data"] / "downloads.sqlite3")
    try:
        connection.execute("DELETE FROM download_artifacts")
        connection.commit()
    finally:
        connection.close()
    result = invoke(paths, "Uninstall", True, effects(root))
    assert_success(result)
    assert configured_data_output.read_text(encoding="utf-8") == "user download"
    assert configured_category_output.read_text(encoding="utf-8") == "user download"
    assert not (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["data"] / "extensions-data" / "runtime.json").read_text(encoding="utf-8") == "owned"


def scenario_changed_or_removed_categories_preserve_old_outputs(root: Path) -> None:
    paths = fixture(root)
    old_data_output = write(paths["data"] / "old-data-category" / "kept.bin", "old download")
    old_cache_output = write(paths["cache"] / "old-cache-category" / "kept.bin", "old download")
    unknown_data = write(paths["data"] / "unknown-user-subtree" / "sentinel.txt", "keep")
    unknown_cache = write(paths["cache"] / "unknown-user-subtree" / "sentinel.txt", "keep")
    known_desktop_log = write(paths["logs"] / "desktop.log.2026-11-08", "owned log")
    known_host_log = write(paths["logs"] / "platform-host.log.2026-11-08.jsonl", "owned log")
    known_failure_log = write(paths["logs"] / "startup-failure.log", "owned log")
    unknown_log = write(paths["logs"] / "kept-output.bin", "keep")
    moved_root = root / "new-download-root"
    write(paths["config"] / "config.json", json.dumps({
        "download": {
            "default_directory": str(moved_root),
            "categories": [{"id": "moved", "name": "Moved", "enabled": True,
                            "save_directory": "Moved", "extensions": [], "url_patterns": []}],
        },
        "clipboard": {"storage_directory": str(paths["clipboard"])},
        "archive": {"temp": {"base_directory": str(paths["archive"])}}
    }))
    connection = sqlite3.connect(paths["data"] / "downloads.sqlite3")
    try:
        connection.execute("DELETE FROM download_artifacts")
        connection.commit()
    finally:
        connection.close()
    result = invoke(paths, "Uninstall", True, effects(root))
    assert_success(result)
    for kept in (old_data_output, old_cache_output, unknown_data, unknown_cache, unknown_log):
        assert kept.read_text(encoding="utf-8") in ("old download", "keep")
    for removed in (known_desktop_log, known_host_log, known_failure_log):
        assert not removed.exists()
    assert not (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["data"] / "extensions-data" / "runtime.json").read_text(encoding="utf-8") == "owned"


def scenario_runtime_subtrees_preserve_unattributed_downloads(root: Path) -> None:
    paths = fixture(root)
    built_clipboard = write(paths["data"] / "clipboard-images" / "downloaded.bin", "download")
    built_extension = write(paths["data"] / "extensions-data" / "downloaded.bin", "download")
    built_archive = write(paths["cache"] / "archive" / "sessions" / "downloaded.bin", "download")
    built_blob = write(paths["data"] / "browser-blob-staging" / "downloaded.bin", "download")
    built_tools = write(paths["data"] / "tools" / "downloaded.bin", "download")
    custom_clipboard = write(paths["clipboard"] / "clipboard-images" / "downloaded.bin", "download")
    custom_archive = write(paths["sessions"] / "downloaded.bin", "download")

    owned_default_asset = paths["data"] / "clipboard-images" / "capture.png"
    owned_custom_asset = paths["clipboard"] / "clipboard-images" / "custom.png"
    write_clipboard_db(paths["data"], owned_default_asset)
    managed_executable = write(paths["data"] / "tools" / "ffmpeg" / "managed-fixture" / "ffmpeg.exe", "managed")
    write_clipboard_db(paths["clipboard"], owned_custom_asset)

    namespace = "11111111-1111-1111-1111-111111111111"
    payload = "22222222-2222-2222-2222-222222222222"
    write(paths["data"] / "extensions-data" / "catalog.json", json.dumps({
        "version": 1,
        "namespaces": {namespace: {"package": "fixture", "state": "active", "file": payload,
                                    "bytes": 2, "revision": 1}},
        "pending": None,
    }))
    write(paths["data"] / "extensions-data" / "initialized", "1")
    write(paths["data"] / "extensions-data" / "quota.lock")
    owned_extension_payload = write(paths["data"] / "extensions-data" / f"{payload}.json", "{}")
    owned_extension_lock = write(paths["data"] / "extensions-data" / f"{namespace}.lock")

    default_session = paths["cache"] / "archive" / "sessions" / "33333333-3333-3333-3333-333333333333"
    custom_session = paths["sessions"] / "44444444-4444-4444-4444-444444444444"
    default_manifest = write(default_session / "manifest.json", json.dumps({
        "Version": 2, "LeaseId": "33333333-3333-3333-3333-333333333333"
    }))
    custom_manifest = write(custom_session / "manifest.json", json.dumps({
        "Version": 2, "LeaseId": "44444444-4444-4444-4444-444444444444"
    }))

    write(paths["config"] / "config.json", json.dumps({
        "download": {"default_directory": str(root / "moved-downloads"), "categories": [],
                     "ffmpeg_executable_path": str(managed_executable)},
        "clipboard": {"storage_directory": str(paths["clipboard"])},
        "archive": {"temp": {"base_directory": str(root / "moved-archive")}},
        "extensions": {},
    }))
    connection = sqlite3.connect(paths["data"] / "downloads.sqlite3")
    try:
        connection.execute("DELETE FROM download_artifacts")
        connection.commit()
    finally:
        connection.close()

    result = invoke(paths, "Uninstall", True, effects(root))
    assert_success(result)
    for sentinel in (built_clipboard, built_extension, built_archive, built_blob, built_tools,
                     custom_clipboard, custom_archive):
        assert sentinel.read_text(encoding="utf-8") == "download"
    for exact_owned in (owned_default_asset, owned_custom_asset, owned_extension_payload,
                        owned_extension_lock, default_manifest, custom_manifest, managed_executable,
                        paths["cache"] / "archive" / "temp-roots" / "0.json"):
        assert not exact_owned.exists()
    assert not (paths["data"] / "clipboard.db").exists()
    assert not (paths["clipboard"] / "clipboard.db").exists()
    assert not (paths["data"] / "extensions-data" / "catalog.json").exists()


def scenario_archive_metadata_delete_failure(root: Path) -> None:
    for metadata_kind in ("session", "retirement"):
        case_root = root / metadata_kind
        paths = fixture(case_root)
        lease = "55555555-5555-5555-5555-555555555555"
        metadata_path = (paths["sessions"] / lease / "manifest.json" if metadata_kind == "session"
                         else paths["sessions"] / f"{lease}.retire.json")
        write(metadata_path, json.dumps({"Version": 2, "LeaseId": lease}))
        effect_path = effects(case_root)
        with effect_path.open("a", encoding="utf-8") as effect_file:
            effect_file.write(r"""
function Remove-NxToolsOwnedFile([string]$Path) {
    if ($Path.EndsWith('manifest.json') -or $Path.EndsWith('.retire.json')) {
        throw [ComponentModel.Win32Exception]::new(32)
    }
    if (Test-Path -LiteralPath $Path -PathType Leaf) { Remove-Item -LiteralPath $Path -Force }
}
""")
        result = invoke(paths, "Uninstall", True, effect_path)
        assert result.returncode != 0, result.stdout
        assert "installer.uninstall_cleanup.failed" in result.stdout
        assert "native_code=32" in result.stdout
        assert "installer.uninstall_cleanup.completed" not in result.stdout
        assert metadata_path.exists()
        assert (paths["data"] / "downloads.sqlite3").exists()


def scenario_upgrade_repair(root: Path, operation: str) -> None:
    paths = fixture(root)
    result = invoke(paths, operation, True, effects(root))
    assert_success(result)
    assert (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["clipboard"] / "clipboard.db").exists()
    assert paths["sessions"].exists()
    assert not (root / "events.txt").exists()


def scenario_stop_failure(root: Path) -> None:
    paths = fixture(root)
    result = invoke(paths, "Uninstall", True, effects(root, fail_process=True))
    assert result.returncode != 0, result.stdout
    assert (paths["data"] / "downloads.sqlite3").exists()
    assert (paths["clipboard"] / "clipboard.db").exists()
    assert paths["sessions"].exists()
    assert (root / "events.txt").read_text(encoding="utf-8").splitlines() == ["processes"]


def scenario_archive_restore_failed(root: Path) -> None:
    paths = fixture(root)
    ledger = write(paths["logs"].parent / "archive-shell-integration-applied.json", "prior choices and owned values")
    effect_path = write(root / "fixture-archive-effects.ps1", """
function Stop-NxToolsOwnedProcesses { Add-Content -LiteralPath $env:NXTOOLS_FIXTURE_EVENTS -Value 'processes' }
function Remove-NxToolsBrowserRegistration { throw 'browser cleanup ran after failed archive restoration' }
function Start-Process {
    param([string]$FilePath, [string]$ArgumentList, [string]$WindowStyle, [switch]$Wait, [switch]$PassThru)
    if (!$Wait -or !$PassThru -or $WindowStyle -ne 'Hidden' -or
        $FilePath -ne (Join-Path $InstallDirectory 'NxTools.PlatformHost.exe') -or
        $ArgumentList -ne "--uninstall-archive-integration $CorrelationId") { throw 'wrong native archive delegation' }
    Add-Content -LiteralPath $env:NXTOOLS_FIXTURE_EVENTS -Value 'native-archive'
    [pscustomobject]@{ ExitCode=11 } | Add-Member -MemberType ScriptMethod -Name Dispose -Value {} -PassThru
}
""")
    result = invoke(paths, "Uninstall", True, effect_path)
    assert result.returncode == 11, result.stdout + result.stderr
    assert "archive_uninstall.terminal" in result.stdout and "exit_code=11" in result.stdout
    assert "installer.uninstall_cleanup.completed" not in result.stdout
    assert ledger.read_text(encoding="utf-8") == "prior choices and owned values"
    assert (paths["config"] / "config.json").exists()
    assert (paths["data"] / "downloads.sqlite3").exists()
    assert (root / "events.txt").read_text(encoding="utf-8").splitlines() == ["processes", "native-archive"]


def installer_contract() -> None:
    source = (ROOT / "installer" / "nxtools-next.iss").read_text(encoding="utf-8")
    initialize = re.search(r"function InitializeUninstall\(\): Boolean;(?P<body>.*?)procedure RunProtectedUninstallCleanup", source, re.S)
    assert initialize, "real uninstaller does not expose the retain/remove choice"
    choice = initialize.group("body")
    assert "UninstallRemovePersonalData := True" in choice
    assert "MB_YESNOCANCEL" in choice and "Choice = IDYES" in choice and "Choice = IDCANCEL" in choice
    callback = re.search(r"procedure CurUninstallStepChanged\(CurUninstallStep: TUninstallStep\);(?P<body>.*?)end;\s*$", source, re.S)
    assert callback, "real uninstall callback is missing"
    body = callback.group("body")
    assert body.index("RunProtectedUninstallCleanup") < body.index("RunUninstallCleanup")
    prepare = re.search(r"function PrepareToInstall.*?(?=procedure FinalizeUpgradeFiles)", source, re.S)
    assert prepare and "RunUninstallCleanup" not in prepare.group(0), "upgrade preparation must preserve personal data"
    assert 'Source: "uninstall-cleanup.ps1"; DestDir: "{app}\\installer-support"' in source

    cleanup = SCRIPT.read_text(encoding="utf-8")
    for forbidden in ("foreach ($root in @($ConfigRoot, $DataRoot, $CacheRoot, $LogRoot)",
                      "Remove-NxToolsTreePreservingPaths", "Remove-Item -LiteralPath $itemPath -Recurse"):
        assert forbidden not in cleanup
    for exact_owner in ("downloads.sqlite3", "ReadClipboardAssets", "Remove-NxToolsExtensionStorage",
                        "Remove-NxToolsArchiveSessionMetadata", "Remove-NxToolsManagedFfmpeg",
                        "desktop.log.*", "platform-host.log.*.jsonl"):
        assert exact_owner in cleanup

def main() -> int:
    installer_contract()
    scratch = Path(os.environ.get("PI_SCRATCH_DIR", tempfile.gettempdir()))
    with tempfile.TemporaryDirectory(prefix="uninstall-cleanup-", dir=scratch) as temporary:
        base = Path(temporary)
        scenarios = [
            ("remove", scenario_remove),
            ("removed-history-configured-roots", scenario_removed_history_configured_roots),
            ("changed-removed-categories", scenario_changed_or_removed_categories_preserve_old_outputs),
            ("runtime-subtree-preservation", scenario_runtime_subtrees_preserve_unattributed_downloads),
            ("retain", scenario_retain),
            ("upgrade", lambda root: scenario_upgrade_repair(root, "Upgrade")),
            ("repair", lambda root: scenario_upgrade_repair(root, "Repair")),
            ("stop-failure", scenario_stop_failure),
            ("archive-metadata-delete-failure", scenario_archive_metadata_delete_failure),
            ("archive-default-restore-failure", scenario_archive_restore_failed),
        ]
        for name, scenario in scenarios:
            scenario(base / name)
        print(f"uninstall cleanup fixtures passed: {len(scenarios)}/{len(scenarios)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
