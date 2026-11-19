use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use crate::DownloadTransferControl;
use crate::http::TransferDirective;

const VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const MUX_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const MAX_CHILD_OUTPUT_BYTES: usize = 4 * 1024;

pub const FFMPEG_GYAN_DOWNLOAD_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
pub const FFMPEG_BTBN_DOWNLOAD_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegResolutionSource {
    Configured,
    Environment,
    Path,
}

impl FfmpegResolutionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Environment => "environment",
            Self::Path => "path",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegResolution {
    pub executable: PathBuf,
    pub source: FfmpegResolutionSource,
}

struct BoundedChildOutput {
    bytes: Vec<u8>,
    overflowed: bool,
}

#[cfg(test)]
mod resolution_tests {
    use super::*;

    #[test]
    fn configured_ffmpeg_wins_over_environment_and_path_without_bundle_fallback() {
        let directory = tempfile::tempdir().expect("create FFmpeg resolution directory");
        let configured = directory.path().join("configured.exe");
        let environment = directory.path().join("environment.exe");
        let path = directory.path().join("path.exe");
        for candidate in [&configured, &environment, &path] {
            std::fs::write(candidate, b"fixture").expect("write FFmpeg candidate");
        }

        let resolved = resolve_ffmpeg_candidates(Some(&configured), Some(&environment), [&path])
            .expect("configured FFmpeg should resolve");

        assert_eq!(resolved.executable, configured);
        assert_eq!(resolved.source, FfmpegResolutionSource::Configured);
    }

    #[test]
    fn invalid_configured_ffmpeg_falls_back_to_environment_then_path() {
        let directory = tempfile::tempdir().expect("create FFmpeg fallback directory");
        let missing = directory.path().join("missing.exe");
        let environment = directory.path().join("environment.exe");
        let path = directory.path().join("path.exe");
        std::fs::write(&environment, b"fixture").expect("write environment candidate");
        std::fs::write(&path, b"fixture").expect("write PATH candidate");

        let environment_result =
            resolve_ffmpeg_candidates(Some(&missing), Some(&environment), [&path])
                .expect("environment FFmpeg should resolve");
        assert_eq!(
            environment_result.source,
            FfmpegResolutionSource::Environment
        );

        std::fs::remove_file(&environment).expect("remove environment candidate");
        let path_result = resolve_ffmpeg_candidates(Some(&missing), Some(&environment), [&path])
            .expect("PATH FFmpeg should resolve");
        assert_eq!(path_result.source, FfmpegResolutionSource::Path);
    }

    #[test]
    fn ffmpeg_resolution_is_missing_when_no_user_or_environment_candidate_exists() {
        let directory = tempfile::tempdir().expect("create missing FFmpeg directory");
        assert!(
            resolve_ffmpeg_candidates(
                Some(&directory.path().join("configured.exe")),
                Some(&directory.path().join("environment.exe")),
                [&directory.path().join("path.exe")],
            )
            .is_none()
        );
    }

    #[test]
    fn ffmpeg_path_environment_variable_may_point_to_an_install_directory() {
        let directory = tempfile::tempdir().expect("create FFmpeg environment directory");
        let executable = directory.path().join(ffmpeg_executable_name());
        std::fs::write(&executable, b"fixture").expect("write environment FFmpeg executable");

        assert_eq!(
            ffmpeg_candidate_from_environment(directory.path()),
            executable
        );
    }

    #[test]
    fn public_download_links_are_https_and_do_not_target_a_bundled_artifact() {
        assert!(FFMPEG_GYAN_DOWNLOAD_URL.starts_with("https://"));
        assert!(FFMPEG_BTBN_DOWNLOAD_URL.starts_with("https://"));
        assert!(!FFMPEG_GYAN_DOWNLOAD_URL.contains("NxTools"));
        assert!(!FFMPEG_BTBN_DOWNLOAD_URL.contains("NxTools"));
    }

    #[test]
    fn public_resolver_observes_ffmpeg_path_and_path_without_application_local_fallback() {
        const CHILD_MARKER: &str = "NXTOOLS_FFMPEG_RESOLUTION_CHILD";
        const EXPECTED_SOURCE: &str = "NXTOOLS_FFMPEG_EXPECTED_SOURCE";
        if std::env::var_os(CHILD_MARKER).is_some() {
            let actual = resolve_ffmpeg_executable(None).map(|resolution| resolution.source);
            let expected = match std::env::var(EXPECTED_SOURCE)
                .expect("child expected source")
                .as_str()
            {
                "environment" => Some(FfmpegResolutionSource::Environment),
                "path" => Some(FfmpegResolutionSource::Path),
                "missing" => None,
                value => panic!("unexpected child source {value}"),
            };
            assert_eq!(actual, expected);
            return;
        }

        let directory = tempfile::tempdir().expect("create public FFmpeg resolver directory");
        let executable = directory.path().join(ffmpeg_executable_name());
        std::fs::write(&executable, b"fixture").expect("write public resolver fixture");
        let current_test = std::env::current_exe().expect("locate current test process");
        let test_name = "media::resolution_tests::public_resolver_observes_ffmpeg_path_and_path_without_application_local_fallback";
        let run_child =
            |child: &Path, expected: &str, ffmpeg_path: Option<&Path>, path: Option<&Path>| {
                let mut command = std::process::Command::new(child);
                command
                    .args(["--exact", test_name, "--nocapture"])
                    .env(CHILD_MARKER, "1")
                    .env(EXPECTED_SOURCE, expected)
                    .env_remove("FFMPEG_PATH");
                if let Some(ffmpeg_path) = ffmpeg_path {
                    command.env("FFMPEG_PATH", ffmpeg_path);
                }
                match path {
                    Some(path) => command.env("PATH", path),
                    None => command.env("PATH", ""),
                };
                assert!(command.status().expect("run resolver child").success());
            };

        run_child(&current_test, "environment", Some(&executable), None);
        run_child(&current_test, "path", None, Some(directory.path()));

        let isolated_directory = tempfile::tempdir().expect("create isolated resolver directory");
        let isolated_test = isolated_directory.path().join("resolver-child.exe");
        std::fs::copy(&current_test, &isolated_test).expect("copy isolated resolver child");
        std::fs::write(
            isolated_directory.path().join(ffmpeg_executable_name()),
            b"application-local fixture must be ignored",
        )
        .expect("write forbidden application-local candidate");
        run_child(&isolated_test, "missing", None, None);
    }
}

async fn drain_bounded_output(
    mut reader: impl AsyncRead + Unpin,
) -> Result<BoundedChildOutput, std::io::Error> {
    let mut bytes = Vec::with_capacity(MAX_CHILD_OUTPUT_BYTES);
    let mut overflowed = false;
    let mut buffer = [0_u8; 1024];
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = MAX_CHILD_OUTPUT_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        overflowed |= read > remaining;
    }
    Ok(BoundedChildOutput { bytes, overflowed })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaProcessOutcome {
    Completed,
    Paused,
    Cancelled,
}

pub fn resolve_ffmpeg_executable(configured: Option<&Path>) -> Option<FfmpegResolution> {
    let environment = std::env::var_os("FFMPEG_PATH")
        .map(PathBuf::from)
        .map(|candidate| ffmpeg_candidate_from_environment(&candidate));
    let path_candidates = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(ffmpeg_executable_name()));
    resolve_ffmpeg_candidates(configured, environment.as_deref(), path_candidates)
}

fn resolve_ffmpeg_candidates<I, P>(
    configured: Option<&Path>,
    environment: Option<&Path>,
    path_candidates: I,
) -> Option<FfmpegResolution>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    if let Some(executable) = configured.filter(|candidate| candidate.is_file()) {
        return Some(FfmpegResolution {
            executable: executable.to_owned(),
            source: FfmpegResolutionSource::Configured,
        });
    }
    if let Some(executable) = environment.filter(|candidate| candidate.is_file()) {
        return Some(FfmpegResolution {
            executable: executable.to_owned(),
            source: FfmpegResolutionSource::Environment,
        });
    }
    path_candidates
        .into_iter()
        .find(|candidate| candidate.as_ref().is_file())
        .map(|executable| FfmpegResolution {
            executable: executable.as_ref().to_owned(),
            source: FfmpegResolutionSource::Path,
        })
}

fn ffmpeg_candidate_from_environment(candidate: &Path) -> PathBuf {
    if candidate.is_dir() {
        candidate.join(ffmpeg_executable_name())
    } else {
        candidate.to_owned()
    }
}

const fn ffmpeg_executable_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

pub(crate) async fn detect_ffmpeg_version(
    executable: &Path,
    control: &DownloadTransferControl,
) -> Result<MediaProcessOutcome, MediaProcessError> {
    let mut child = Command::new(executable)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(MediaProcessError::Spawn)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(MediaProcessError::MissingOutput)?;
    let output_reader = tokio::spawn(drain_bounded_output(stdout));
    let deadline = Instant::now() + VERSION_TIMEOUT;
    let status = loop {
        if Instant::now() >= deadline {
            child.kill().await.map_err(MediaProcessError::Wait)?;
            let _ = child.wait().await;
            output_reader.abort();
            return Err(MediaProcessError::Timeout {
                operation: "version",
            });
        }
        tokio::select! {
            status = child.wait() => break status.map_err(MediaProcessError::Wait)?,
            directive = control.wait_for_stop_or_timeout(Duration::from_millis(50)) => {
                match directive {
                    Some(TransferDirective::Pause) => {
                        child.kill().await.map_err(MediaProcessError::Wait)?;
                        let _ = child.wait().await;
                        output_reader.abort();
                        return Ok(MediaProcessOutcome::Paused);
                    }
                    Some(TransferDirective::Cancel) => {
                        child.kill().await.map_err(MediaProcessError::Wait)?;
                        let _ = child.wait().await;
                        output_reader.abort();
                        return Ok(MediaProcessOutcome::Cancelled);
                    }
                    Some(TransferDirective::Running) | None => {}
                }
            }
        }
    };
    let output = output_reader
        .await
        .map_err(|_| MediaProcessError::OutputReader)?
        .map_err(MediaProcessError::Wait)?;
    if !status.success() {
        return Err(MediaProcessError::Exit {
            operation: "version",
            code: status.code(),
        });
    }
    if output.overflowed {
        return Err(MediaProcessError::OutputTooLarge);
    }
    let first_line = std::str::from_utf8(&output.bytes)
        .map_err(|_| MediaProcessError::InvalidVersion)?
        .lines()
        .next()
        .ok_or(MediaProcessError::InvalidVersion)?;
    first_line
        .strip_prefix("ffmpeg version ")
        .and_then(|line| line.split_whitespace().next())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
        .ok_or(MediaProcessError::InvalidVersion)?;
    Ok(MediaProcessOutcome::Completed)
}

pub(crate) async fn mux_dash_tracks(
    executable: &Path,
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
    control: &DownloadTransferControl,
    progress: impl Fn(u64, bool) + Send + 'static,
) -> Result<MediaProcessOutcome, MediaProcessError> {
    mux_dash_tracks_with_timeout(
        executable,
        video_path,
        audio_path,
        output_path,
        control,
        MUX_TIMEOUT,
        progress,
    )
    .await
}

async fn mux_dash_tracks_with_timeout(
    executable: &Path,
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
    control: &DownloadTransferControl,
    timeout: Duration,
    progress: impl Fn(u64, bool) + Send + 'static,
) -> Result<MediaProcessOutcome, MediaProcessError> {
    let mut child = Command::new(executable)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostats",
            "-progress",
            "pipe:1",
            "-y",
            "-i",
        ])
        .arg(video_path)
        .arg("-i")
        .arg(audio_path)
        .args(["-map", "0:v:0", "-map", "1:a:0", "-c", "copy", "-f", "mp4"])
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(MediaProcessError::Spawn)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(MediaProcessError::MissingOutput)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(MediaProcessError::MissingOutput)?;
    let error_reader = tokio::spawn(drain_bounded_output(stderr));
    let progress_reader = tokio::spawn(read_mux_progress(stdout, progress));
    let deadline = Instant::now() + timeout;
    let outcome = loop {
        if Instant::now() >= deadline {
            break Err(MediaProcessError::Timeout { operation: "mux" });
        }
        tokio::select! {
            status = child.wait() => {
                break match status {
                    Ok(status) if status.success() => Ok(MediaProcessOutcome::Completed),
                    Ok(status) => Err(MediaProcessError::Exit { operation: "mux", code: status.code() }),
                    Err(error) => Err(MediaProcessError::Wait(error)),
                };
            }
            directive = control.wait_for_stop_or_timeout(Duration::from_millis(50)) => {
                match directive {
                    Some(TransferDirective::Pause) => break Ok(MediaProcessOutcome::Paused),
                    Some(TransferDirective::Cancel) => break Ok(MediaProcessOutcome::Cancelled),
                    Some(TransferDirective::Running) | None => {}
                }
            }
        }
    };
    if !matches!(outcome, Ok(MediaProcessOutcome::Completed)) {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    let (error_result, progress_result) = tokio::join!(error_reader, progress_reader);
    let error_result = error_result.map_err(|_| MediaProcessError::OutputReader)?;
    let progress_result = progress_result.map_err(|_| MediaProcessError::OutputReader)?;
    let error_output = error_result.map_err(MediaProcessError::Wait)?;
    progress_result.map_err(MediaProcessError::Wait)?;
    match outcome? {
        MediaProcessOutcome::Completed => {
            if error_output.overflowed {
                Err(MediaProcessError::OutputTooLarge)
            } else {
                Ok(MediaProcessOutcome::Completed)
            }
        }
        stopped => Ok(stopped),
    }
}

async fn read_mux_progress(
    mut stdout: impl AsyncRead + Unpin,
    progress: impl Fn(u64, bool),
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 4096];
    let mut line = Vec::with_capacity(64);
    let mut current = 0;
    loop {
        let count = stdout.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        for byte in &buffer[..count] {
            if *byte == b'\n' {
                let record = line.strip_suffix(b"\r").unwrap_or(&line);
                if let Some(value) = record.strip_prefix(b"out_time_us=")
                    && let Ok(text) = std::str::from_utf8(value)
                    && let Ok(micros) = text.trim().parse::<u64>()
                {
                    current = micros;
                    progress(micros, false);
                } else if record == b"progress=end" {
                    progress(current, true);
                }
                line.clear();
            } else if line.len() < 128 {
                line.push(*byte);
            }
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fragmented_mux_progress_preserves_the_windows_terminal_record() {
        let (mut writer, reader) = tokio::io::duplex(4);
        let output = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            for part in [
                b"out_time_".as_slice(),
                b"us=125000\r\npro",
                b"gress=end\r\n",
            ] {
                writer.write_all(part).await.expect("write split progress");
            }
        });
        let samples = std::sync::Mutex::new(Vec::new());
        read_mux_progress(reader, |micros, terminal| {
            samples
                .lock()
                .expect("progress samples")
                .push((micros, terminal));
        })
        .await
        .expect("read bounded progress records");
        output.await.expect("progress writer settled");
        assert_eq!(
            samples.into_inner().expect("progress samples"),
            vec![(125000, false), (125000, true)]
        );
    }

    #[tokio::test]
    async fn mux_timeout_kills_a_real_child_without_publishing_output() {
        let directory = tempfile::tempdir().expect("create FFmpeg timeout directory");
        let executable = directory.path().join("ffmpeg-timeout.cmd");
        std::fs::write(
            &executable,
            r#"@echo off
:wait
ping -n 2 127.0.0.1 >nul
goto wait
"#,
        )
        .expect("write FFmpeg timeout fixture");
        let video = directory.path().join("video.track");
        let audio = directory.path().join("audio.track");
        let output = directory.path().join("output.part");
        std::fs::write(&video, b"video").expect("write video fixture");
        std::fs::write(&audio, b"audio").expect("write audio fixture");

        let error = mux_dash_tracks_with_timeout(
            &executable,
            &video,
            &audio,
            &output,
            &DownloadTransferControl::new(),
            Duration::from_millis(100),
            |_, _| {},
        )
        .await
        .expect_err("bounded FFmpeg process should time out");
        assert!(matches!(
            error,
            MediaProcessError::Timeout { operation: "mux" }
        ));
        assert!(!output.exists());
    }

    #[tokio::test]
    async fn oversized_mux_stderr_does_not_hide_the_real_exit_code() {
        let directory = tempfile::tempdir().expect("create FFmpeg exit directory");
        let executable = directory.path().join("ffmpeg-exit.cmd");
        std::fs::write(
            &executable,
            r#"@echo off
for /L %%i in (1,1,5000) do <nul set /p =x 1>&2
exit /b 7
"#,
        )
        .expect("write FFmpeg exit fixture");
        let video = directory.path().join("video.track");
        let audio = directory.path().join("audio.track");
        let output = directory.path().join("output.part");
        std::fs::write(&video, b"video").expect("write video fixture");
        std::fs::write(&audio, b"audio").expect("write audio fixture");

        let error = mux_dash_tracks_with_timeout(
            &executable,
            &video,
            &audio,
            &output,
            &DownloadTransferControl::new(),
            Duration::from_secs(5),
            |_, _| {},
        )
        .await
        .expect_err("nonzero FFmpeg exit should fail");
        assert!(matches!(
            error,
            MediaProcessError::Exit {
                operation: "mux",
                code: Some(7)
            }
        ));
    }

    #[tokio::test]
    async fn oversized_version_stdout_does_not_hide_the_real_exit_code() {
        let directory = tempfile::tempdir().expect("create FFmpeg version exit directory");
        let executable = directory.path().join("ffmpeg-version-exit.cmd");
        std::fs::write(
            &executable,
            r#"@echo off
for /L %%i in (1,1,5000) do <nul set /p =x
exit /b 6
"#,
        )
        .expect("write FFmpeg version exit fixture");
        let error = detect_ffmpeg_version(&executable, &DownloadTransferControl::new())
            .await
            .expect_err("nonzero FFmpeg version exit should fail");
        assert!(matches!(
            error,
            MediaProcessError::Exit {
                operation: "version",
                code: Some(6)
            }
        ));
    }
}

#[derive(Debug, Error)]
pub(crate) enum MediaProcessError {
    #[error("FFmpeg could not be started")]
    Spawn(#[source] std::io::Error),
    #[error("FFmpeg process wait failed")]
    Wait(#[source] std::io::Error),
    #[error("FFmpeg did not expose bounded version output")]
    MissingOutput,
    #[error("FFmpeg version output reader failed")]
    OutputReader,
    #[error("FFmpeg child output exceeded its bounded capture limit")]
    OutputTooLarge,
    #[error("FFmpeg version output was invalid")]
    InvalidVersion,
    #[error("FFmpeg {operation} exceeded its time limit")]
    Timeout { operation: &'static str },
    #[error("FFmpeg {operation} failed with exit code {code:?}")]
    Exit {
        operation: &'static str,
        code: Option<i32>,
    },
}

impl MediaProcessError {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Spawn(_) => "spawn",
            Self::Wait(_) | Self::OutputReader => "io",
            Self::MissingOutput | Self::InvalidVersion => "version",
            Self::OutputTooLarge => "output",
            Self::Timeout { .. } => "timeout",
            Self::Exit { .. } => "exit",
        }
    }

    pub(crate) fn native_error_code(&self) -> Option<i32> {
        match self {
            Self::Spawn(error) | Self::Wait(error) => error.raw_os_error(),
            _ => None,
        }
    }

    pub(crate) fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Exit { code, .. } => *code,
            _ => None,
        }
    }
}
