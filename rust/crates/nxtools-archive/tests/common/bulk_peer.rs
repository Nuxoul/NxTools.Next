//! Test-only adapter from mixed private frames to existing native-peer assertions.
//! Payload arrays/base64 exist only inside the fixture; modern pipe traffic is raw.
use base64::Engine as _;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, Read, Write},
};
use uuid::Uuid;

struct Peer {
    operation: Uuid,
    accepted: bool,
    pipelined_output: bool,
    sequence: u64,
    pending: Option<Value>,
    written: HashMap<Uuid, u64>,
}
thread_local! {
    static PEER: RefCell<Option<Peer>> = const { RefCell::new(None) };
}

fn id(value: &Value) -> Uuid {
    let id = Uuid::parse_str(value.as_str().expect("UUID string")).expect("RFC4122 UUID");
    assert!(!id.is_nil(), "nonempty original identity");
    id
}

fn admit(peer: &mut Peer, call: &Value) {
    assert!(peer.accepted, "bulk capability precedes native effects");
    assert!(peer.pending.is_none(), "one acknowledged native call");
    id(&call["call_id"]);
    if matches!(
        call["action"].as_str(),
        Some("write" | "source_read" | "read")
    ) {
        id(&call["token"]);
        peer.sequence += 1;
        assert_eq!(
            call["sequence"].as_u64(),
            Some(peer.sequence),
            "operation-wide sequence"
        );
        assert!((1..=65536).contains(&call["length"].as_u64().expect("requested length")));
        call["offset"].as_u64().expect("byte offset");
    }
    peer.pending = Some(call.clone());
}

pub fn read_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> io::Result<Option<T>> {
    let mut prefix = [0; 4];
    if reader.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix);
    let binary = length & 0x8000_0000 != 0;
    let size = (length & 0x7fff_ffff) as usize;
    if !(if binary { 76..=65612 } else { 1..=1048576 }).contains(&size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "private frame length",
        ));
    }
    let mut body = vec![0; size];
    reader.read_exact(&mut body)?;
    let frame: Value = if binary {
        assert_eq!(&body[..4], &[1, 1, 0, 0], "version/kind/direction/reserved");
        let operation = Uuid::from_slice(&body[4..20]).expect("operation UUID bytes");
        let call = Uuid::from_slice(&body[20..36]).expect("call UUID bytes");
        let token = Uuid::from_slice(&body[36..52]).expect("token UUID bytes");
        assert!(!operation.is_nil() && !call.is_nil() && !token.is_nil());
        let sequence = u64::from_le_bytes(body[52..60].try_into().expect("sequence bytes"));
        let offset = u64::from_le_bytes(body[60..68].try_into().expect("offset bytes"));
        let requested = u32::from_le_bytes(body[68..72].try_into().expect("requested bytes"));
        let actual = u32::from_le_bytes(body[72..76].try_into().expect("actual bytes"));
        assert_eq!(requested, actual, "full output write");
        assert_eq!(size, 76 + actual as usize, "exact binary body");
        let native = json!({"action":"write","call_id":call,"token":token,
            "sequence":sequence,"offset":offset,"length":requested,"data":&body[76..]});
        PEER.with_borrow_mut(|state| {
            let peer = state.as_mut().expect("explicit bulk negotiation");
            assert_eq!(operation, peer.operation, "original operation");
            assert_eq!(
                peer.written.get(&token),
                Some(&offset),
                "owned token/write offset"
            );
            admit(peer, &native);
        });
        json!({"version":1,"operation_id":operation,"op":"native_call","native":native})
    } else {
        let frame: Value = serde_json::from_slice(&body)?;
        PEER.with_borrow_mut(|state| {
            let Some(peer) = state.as_mut() else {
                return;
            };
            assert_eq!(
                id(&frame["operation_id"]),
                peer.operation,
                "original operation"
            );
            match frame["op"].as_str() {
                Some("accepted") => {
                    assert!(!peer.accepted, "single admission");
                    let capabilities = frame["capabilities"].as_array().expect("capability array");
                    assert!(
                        capabilities.iter().all(Value::is_string),
                        "typed capabilities"
                    );
                    assert_eq!(
                        capabilities
                            .iter()
                            .filter(|value| *value == "binary_bulk_v1")
                            .count(),
                        1
                    );
                    peer.accepted = true;
                    peer.pipelined_output = capabilities
                        .iter()
                        .any(|value| value == "extraction_output_v1");
                }
                Some("native_call") => {
                    assert_ne!(
                        frame["native"]["action"], "write",
                        "modern writes are binary only"
                    );
                    admit(peer, &frame["native"]);
                }
                Some("complete" | "failure") => *state = None,
                _ => {}
            }
        });
        frame
    };
    serde_json::from_value(frame)
        .map(Some)
        .map_err(io::Error::other)
}

pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    let mut frame = serde_json::to_value(value)?;
    if frame["op"] == "start" {
        PEER.with_borrow_mut(|state| {
            *state = if frame["bulk_transfer"] == 1 {
                assert_eq!(frame["schema"], 2);
                Some(Peer {
                    operation: id(&frame["operation_id"]),
                    accepted: false,
                    pipelined_output: false,
                    sequence: 0,
                    pending: None,
                    written: HashMap::new(),
                })
            } else {
                None
            };
        });
    }
    let mut raw = None;
    let mut suppress_ack = false;
    if frame["op"] == "native_result" {
        PEER.with_borrow_mut(|state| {
            let Some(peer) = state.as_mut() else {
                return;
            };
            assert_eq!(id(&frame["operation_id"]), peer.operation);
            let call = peer.pending.take().expect("current native rendezvous");
            assert_eq!(
                frame["call_id"], call["call_id"],
                "exact call acknowledgement"
            );
            let success = frame["success"] == true;
            match call["action"].as_str().expect("native action") {
                "begin" if success => {
                    assert!(peer.written.insert(id(&frame["token"]), 0).is_none());
                }
                "commit" | "abort" if success => {
                    peer.written.remove(&id(&call["token"]));
                }
                "write" | "source_read" | "read" => {
                    for field in ["token", "sequence", "offset", "length"] {
                        frame[field] = call[field].clone();
                    }
                    if call["action"] == "write" {
                        if success {
                            peer.written.insert(
                                id(&call["token"]),
                                call["offset"].as_u64().expect("offset")
                                    + call["length"].as_u64().expect("length"),
                            );
                        }
                        suppress_ack = peer.pipelined_output;
                    } else if success {
                        let payload = base64::engine::general_purpose::STANDARD
                            .decode(frame["read_data"].as_str().expect("fixture read bytes"))
                            .expect("fixture base64");
                        let mut header = Vec::with_capacity(76);
                        header.extend_from_slice(&[
                            1,
                            if call["action"] == "source_read" {
                                2
                            } else {
                                3
                            },
                            0,
                            0,
                        ]);
                        header.extend_from_slice(peer.operation.as_bytes());
                        header.extend_from_slice(id(&call["call_id"]).as_bytes());
                        header.extend_from_slice(id(&call["token"]).as_bytes());
                        header.extend_from_slice(
                            &call["sequence"].as_u64().expect("sequence").to_le_bytes(),
                        );
                        header.extend_from_slice(
                            &call["offset"].as_u64().expect("offset").to_le_bytes(),
                        );
                        let requested = u32::try_from(call["length"].as_u64().expect("length"))
                            .expect("u32 length");
                        assert!(payload.len() <= requested as usize, "short read or EOF");
                        header.extend_from_slice(&requested.to_le_bytes());
                        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                        assert_eq!(header.len(), 76);
                        raw = Some((header, payload));
                    }
                    frame
                        .as_object_mut()
                        .expect("reply object")
                        .remove("read_data");
                }
                _ => {}
            }
        });
    }
    if suppress_ack {
        return Ok(());
    }
    if let Some((header, payload)) = raw {
        writer.write_all(&(0x8000_0000 | (76 + payload.len()) as u32).to_le_bytes())?;
        writer.write_all(&header)?;
        writer.write_all(&payload)?;
        writer.flush()
    } else {
        nxtools_archive::worker::write_frame(writer, &frame)
    }
}
