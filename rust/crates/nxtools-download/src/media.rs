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

struct BoundedChildOutput {
    bytes: Vec<u8>,
    overflowed: bool,
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

pub(crate) fn default_ffmpeg_executable() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("ffmpeg.exe")))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
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
) -> Result<MediaProcessOutcome, MediaProcessError> {
    mux_dash_tracks_with_timeout(
        executable,
        video_path,
        audio_path,
        output_path,
        control,
        MUX_TIMEOUT,
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
) -> Result<MediaProcessOutcome, MediaProcessError> {
    let mut child = Command::new(executable)
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(video_path)
        .arg("-i")
        .arg(audio_path)
        .args(["-map", "0:v:0", "-map", "1:a:0", "-c", "copy", "-f", "mp4"])
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(MediaProcessError::Spawn)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(MediaProcessError::MissingOutput)?;
    let output_reader = tokio::spawn(drain_bounded_output(stderr));
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            child.kill().await.map_err(MediaProcessError::Wait)?;
            let _ = child.wait().await;
            output_reader.abort();
            return Err(MediaProcessError::Timeout { operation: "mux" });
        }
        tokio::select! {
            status = child.wait() => {
                let status = status.map_err(MediaProcessError::Wait)?;
                let output = output_reader
                    .await
                    .map_err(|_| MediaProcessError::OutputReader)?
                    .map_err(MediaProcessError::Wait)?;
                if !status.success() {
                    return Err(MediaProcessError::Exit { operation: "mux", code: status.code() });
                }
                return if output.overflowed {
                    Err(MediaProcessError::OutputTooLarge)
                } else {
                    Ok(MediaProcessOutcome::Completed)
                };
            }
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
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

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
