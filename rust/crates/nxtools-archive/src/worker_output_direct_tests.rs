use super::{NativeReply, PrivateOutput, ReplyPort};
use crate::{OperationControl, output::ArchiveOutput, worker_bulk::Packet};
use std::{io::Write, sync::mpsc, time::Duration};
use uuid::Uuid;

struct Wire(mpsc::SyncSender<Vec<u8>>, Vec<u8>);
impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.1.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0
            .send(std::mem::take(&mut self.1))
            .map_err(|_| std::io::ErrorKind::BrokenPipe.into())
    }
}
fn reply(port: &ReplyPort, call: Uuid, token: Option<Uuid>) {
    assert!(port.deliver(NativeReply {
        call_id: call,
        success: true,
        token,
        error_code: None,
        read_data: None,
        existing_bytes: None,
        sequence: None,
        offset: None,
        length: None,
        binary: None,
        payload: None,
    }));
}

#[test]
fn new_files_use_only_begin_and_close_without_payload_acknowledgements() {
    const FILES: usize = 3;
    let control = OperationControl::default();
    let port = ReplyPort::new();
    port.configure_bulk(true);
    // Capacity one also exercises producer backpressure without an unbounded test queue.
    let (send, receive) = mpsc::sync_channel(1);
    let worker = std::thread::spawn({
        let control = control.clone();
        let port = port.clone();
        move || {
            let mut wire = Wire(send, Vec::new());
            let mut output = PrivateOutput::new(&mut wire, &control, port)
                .ordinary_extraction(true)
                .extraction_metadata(true);
            for index in 0..FILES {
                let token = output
                    .begin_extracted_file(&format!("file{index}.bin"), Some(65_539))?
                    .expect("new file");
                output.write_file(token, &[0x51; 65_536])?;
                output.write_file(token, &[7, 8, 9])?;
                output.set_file_metadata(token, crate::metadata::OutputMetadata::default())?;
                output.commit_file(token)?;
            }
            crate::Result::Ok(())
        }
    });
    let mut calls = Vec::new();
    let mut writes = 0;
    let mut sequence = 0;
    let mut bytes = 0;
    let mut reached_close = false;
    let mut active = None;
    while let Ok(frame) = receive.recv_timeout(Duration::from_secs(3)) {
        match crate::worker_bulk::read(&mut frame.as_slice())
            .expect("wire")
            .expect("packet")
        {
            Packet::Binary(binding, data) => {
                sequence += 1;
                assert_eq!(binding.sequence, sequence);
                assert_eq!(Some(binding.token), active);
                assert_eq!(binding.offset, bytes);
                assert!(data.len() <= 32_768);
                bytes += data.len() as u64;
                writes += 1;
                // No native_result for a pipelined write. Close is the ordered drain barrier.
            }
            Packet::Json(body) => {
                let frame: serde_json::Value = serde_json::from_slice(&body).expect("JSON");
                let native = &frame["native"];
                let action = native["action"].as_str().expect("native action");
                calls.push(action.to_owned());
                let call =
                    Uuid::parse_str(native["call_id"].as_str().expect("call")).expect("UUID");
                if action == "begin" {
                    active = Some(Uuid::new_v4());
                    bytes = 0;
                    reply(&port, call, active);
                } else {
                    assert_eq!(action, "commit");
                    assert_eq!(bytes, 65_539);
                    assert!(
                        native["metadata"].is_object(),
                        "close carries file metadata"
                    );
                    reached_close = true;
                    reply(&port, call, None);
                    if calls.len() == FILES * 2 {
                        break;
                    }
                }
            }
        }
    }
    if calls.len() != FILES * 2 {
        control.cancel();
    }
    let result = worker.join().expect("worker output thread");
    assert!(
        reached_close,
        "write must not wait for per-block acknowledgement"
    );
    assert!(result.is_ok(), "complete output: {result:?}");
    assert_eq!(calls, ["begin", "commit"].repeat(FILES));
    assert_eq!(writes, FILES * 3);
    let work = control.work_observation();
    assert_eq!(work.output_begin_calls, FILES as u64);
    assert_eq!(work.output_close_calls, FILES as u64);
    assert_eq!(work.output_directory_calls, 0);
    assert_eq!(work.output_directory_metadata_calls, 0);
    assert_eq!(work.output_write_calls, writes as u64);
    assert!(work.output_begin_wait_us > 0 && work.output_close_wait_us > 0);
}
