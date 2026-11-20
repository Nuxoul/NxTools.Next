//! One installer-originated Apply through the existing platform command/result router.
use super::*;
use nxtools_protocol::archive_shell_integration::{
    ArchiveShellIntegrationAction, ArchiveShellIntegrationCommand,
};
use std::os::windows::io::AsRawHandle;

const READY_TIMEOUT: Duration = Duration::from_secs(8);
pub(super) const NATIVE_REPLY_TIMEOUT: Duration = Duration::from_secs(20);

pub fn apply(
    pipe_name: &str,
    correlation_id: uuid::Uuid,
    command: ArchiveShellIntegrationCommand,
) -> Result<PlatformCommandResult> {
    anyhow::ensure!(
        command.action == ArchiveShellIntegrationAction::Apply && command.validate().is_ok(),
        "invalid installer archive command"
    );
    let pipe = format!(r"\\.\pipe\{pipe_name}");
    let mut connection = connect_local_pipe(&pipe, READY_TIMEOUT)?;
    local_pipe::authorize_desktop_peer(connection.writer.as_raw_handle(), false)?;
    let mut request = Envelope::new(Message::PlatformCommand(
        PlatformCommand::ArchiveShellIntegration(command),
    ));
    request.id = correlation_id;
    let encoded = request.encode_line()?;
    local_pipe::write_desktop_frame(
        &mut connection.writer,
        encoded.as_bytes(),
        Duration::from_secs(3),
    )?;
    connection
        .reader
        .get_mut()
        .set_initial_frame_timeout(READY_TIMEOUT + NATIVE_REPLY_TIMEOUT + Duration::from_secs(3));
    let frame = read_bounded_sync_frame(&mut connection.reader)?
        .context("missing archive Apply acknowledgement")?;
    let response = Envelope::decode_line(&frame)?;
    anyhow::ensure!(
        response.correlation_id == Some(correlation_id),
        "unexpected archive Apply correlation"
    );
    match response.message {
        Message::PlatformCommandResult(result) => Ok(result),
        _ => anyhow::bail!("unexpected archive Apply acknowledgement"),
    }
}

pub(super) async fn serve(
    request: Envelope,
    endpoint: &PlatformServerEndpoint,
    desktop_authorized: bool,
    writer: &mut (impl tokio::io::AsyncWrite + Unpin),
) -> Result<()> {
    let valid = desktop_authorized
        && !request.id.is_nil()
        && request.correlation_id.is_none()
        && matches!(&request.message, Message::PlatformCommand(PlatformCommand::ArchiveShellIntegration(command))
            if command.action == ArchiveShellIntegrationAction::Apply && command.validate().is_ok());
    let result = if valid {
        request_apply(request.clone(), endpoint).await
    } else {
        failed_platform_result("invalid installer archive command or Desktop peer")
    };
    let response = Envelope::reply_to(&request, Message::PlatformCommandResult(result));
    tokio::time::timeout(Duration::from_secs(2), write_envelope(writer, &response)).await??;
    Ok(())
}

async fn request_apply(
    request: Envelope,
    endpoint: &PlatformServerEndpoint,
) -> PlatformCommandResult {
    let readiness = tokio::time::timeout(READY_TIMEOUT, async {
        while !endpoint.connected.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    if readiness.is_err() {
        return failed_platform_result("archive platform readiness expired");
    }
    let (reply, terminal) = async_channel::bounded(1);
    if let Err(error) = endpoint.send_summon_command(request, reply) {
        return failed_platform_result(error.to_string());
    }
    // The ordinary generation-bound router supplies native/error/timeout/disconnect terminals.
    match tokio::time::timeout(
        NATIVE_REPLY_TIMEOUT + Duration::from_secs(2),
        terminal.recv(),
    )
    .await
    {
        Ok(Ok(result)) => result,
        _ => failed_platform_result("archive Apply result was not confirmed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(action: ArchiveShellIntegrationAction) -> Envelope {
        Envelope::new(Message::PlatformCommand(
            PlatformCommand::ArchiveShellIntegration(ArchiveShellIntegrationCommand {
                version: 1,
                action,
                desired_revision: 1,
                classic_cascade: false,
                open_with: false,
                extensions: vec![],
                double_click_action: "open".into(),
                set_default: false,
                menu_layout: "cascade".into(),
                menu_commands: vec![],
            }),
        ))
    }

    #[tokio::test]
    async fn installed_archive_apply_waits_for_readiness_and_original_terminal() {
        let (_bridge, endpoint) = platform_bridge(4);
        let envelope = request(ArchiveShellIntegrationAction::Apply);
        let id = envelope.id;
        let requester_endpoint = endpoint.clone();
        let waiter =
            tokio::spawn(async move { request_apply(envelope, &requester_endpoint).await });
        tokio::task::yield_now().await;
        assert!(
            !waiter.is_finished(),
            "no readiness is not a terminal acknowledgement"
        );
        let (generation, receiver, _) = endpoint.open_connection();
        assert_eq!(
            receiver.recv().await.expect("forwarded Apply command").id,
            id
        );
        assert!(
            !waiter.is_finished(),
            "queue admission is not Apply success"
        );
        assert!(endpoint.complete_command(generation + 1, id).is_none());
        let mut terminal = failed_platform_result("injected native failure");
        terminal.native_error_code = Some(5);
        let pending = endpoint
            .complete_command(generation, id)
            .expect("matching pending Apply");
        pending
            .summon_reply
            .expect("original installer reply")
            .send(terminal)
            .await
            .expect("deliver native terminal");
        let observed = waiter.await.expect("installer waiter");
        assert!(!observed.success);
        assert_eq!(observed.native_error_code, Some(5));
    }

    #[tokio::test]
    async fn installed_archive_apply_refuses_unproved_peers_and_non_apply_commands() {
        let (_bridge, endpoint) = platform_bridge(4);
        let (_, receiver, _) = endpoint.open_connection();
        for (action, authorized) in [
            (ArchiveShellIntegrationAction::Apply, false),
            (ArchiveShellIntegrationAction::Revert, true),
        ] {
            let request = request(action);
            let id = request.id;
            let mut output = Vec::new();
            serve(request, &endpoint, authorized, &mut output)
                .await
                .expect("serve rejected installer request");
            let response =
                Envelope::decode_line(std::str::from_utf8(&output).expect("response UTF-8"))
                    .expect("rejected installer response");
            assert_eq!(response.correlation_id, Some(id));
            assert!(
                matches!(response.message, Message::PlatformCommandResult(result) if !result.success)
            );
            assert!(receiver.try_recv().is_err());
        }
    }
}
