//! Small independent fixtures at the real TAR edit worker rendezvous (not native publication).
#![cfg(feature = "isolated-codecs")]
use base64::Engine as _;
use nxtools_archive::worker::{read_frame, write_frame};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};
use uuid::Uuid;

const FIXTURE: &str = r#"
import io,sys,tarfile,json,gzip,lzma
path,fmt,recipe=sys.argv[1:]
mode={'Tar':'w','TarGz':'w:gz','TarXz':'w:xz'}[fmt]
style=tarfile.GNU_FORMAT if recipe=='gnu' else tarfile.PAX_FORMAT if recipe in ('pax','unknown','global') else tarfile.USTAR_FORMAT
with tarfile.open(path,mode,format=style,pax_headers={'comment':'global'} if recipe=='global' else None) as t:
    names=['a/','a/one','a/sub/','a/sub/two','ab/keep','empty/','zero']
    if recipe=='duplicate': names=['a','a']
    if recipe=='case': names=['a','A']
    if recipe=='prefix': names=['a','a/child']
    if recipe=='unsafe': names=['../escape']
    if recipe=='gnu': names=['long/'+'segment/'*22+'file']
    if recipe in ('pax','unknown','global'): names=['old']
    if recipe=='link': names=['link']
    for i,name in enumerate(names):
        h=tarfile.TarInfo(name); h.uid=123; h.gid=456; h.mode=0o750 if name.endswith('/') else 0o640; h.mtime=1700000000; h.uname='owner'; h.gname='group'
        if name.endswith('/'): h.type=tarfile.DIRTYPE; data=b''
        else: data=b'' if name=='zero' else ('data:'+name).encode()
        if recipe=='link': h.type=tarfile.SYMTYPE; h.linkname='target'; data=b''
        if recipe=='pax': h.pax_headers={'mtime':'1700000000.123456789','atime':'1700000001.987654321','ctime':'-1.25','uid':'12345678','gid':'87654321','uname':'用户','gname':'组'}
        if recipe=='unknown': h.pax_headers={'SCHILY.xattr.user.secret':'must-not-drop'}
        h.size=len(data); t.addfile(h,io.BytesIO(data))
if recipe=='tail':
    with open(path,'ab') as f: f.write(b'JUNK')
if recipe=='truncated':
    b=open(path,'rb').read(); open(path,'wb').write(b[:-9])
if recipe=='multi':
    b=open(path,'rb').read(); open(path,'ab').write(b)
"#;
const ORACLE: &str = r#"
import sys,tarfile,json,hashlib
with tarfile.open(sys.argv[1]) as t:
    result={}
    for h in t:
        assert h.name not in result
        result[h.name]={'content':None if h.isdir() else t.extractfile(h).read().decode(), 'uid':h.uid,'gid':h.gid,'mode':h.mode,'mtime':h.mtime,'uname':h.uname,'gname':h.gname,'pax':h.pax_headers}
    print(json.dumps(result,ensure_ascii=True))
"#;
#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    Write,
    Read,
    SourceChange,
    SourceEof,
    SourceExtra,
    Commit5,
    Commit64,
    CancelBeforeHandoff,
}
struct Outcome {
    terminal: Value,
    members: Value,
    commits: usize,
    begins: usize,
    handed: bool,
}
fn run(
    format: &str,
    recipe: &str,
    mut edit: Value,
    incoming: bool,
    fault: Fault,
    policy: Option<&str>,
    maximum: Option<u64>,
) -> Outcome {
    let root = tempfile::tempdir().expect("owned fixtures");
    let path = root.path().join("owned.tar");
    assert!(
        Command::new("python")
            .args(["-c", FIXTURE])
            .arg(&path)
            .args([format, recipe])
            .status()
            .expect("independent fixture")
            .success()
    );
    let original = std::fs::read(&path).expect("original");
    let digest: [u8; 32] = Sha256::digest(&original).into();
    let metadata = std::fs::metadata(&path).expect("metadata");
    edit["browser"] = json!({"snapshot_id":Uuid::new_v4(),"format":format,"comment":"","source":{"native":"v1:fixture","digest":digest,"size":metadata.len(),"modified_unix_ns":metadata.modified().expect("mtime").duration_since(UNIX_EPOCH).expect("epoch").as_nanos() as u64}});
    let operation = Uuid::new_v4();
    let token = Uuid::new_v4();
    let source_id = Uuid::new_v4();
    let directory_id = Uuid::new_v4();
    let empty_id = Uuid::new_v4();
    let zero_id = Uuid::new_v4();
    let incoming_name = "incoming";
    let mut request = json!({"schema":2,"operation_id":operation,"op":"start","operation":"modify_tar","path":path.to_str().expect("path"),"job_execution":true,"password_interaction":true,"job_modify":edit});
    if let Some(policy) = policy {
        request["conflict_policy"] = json!(policy);
    }
    if let Some(maximum) = maximum {
        request["limits"] = json!({"max_output_bytes":maximum});
    }
    let mut partial = tempfile::tempfile().expect("partial");
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("worker");
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let mut commits = 0;
    let mut begins = 0;
    let mut handed = false;
    let mut opened = 0;
    let mut closed = 0;
    let terminal = loop {
        let Some(frame) =
            read_frame::<_, Value>(child.stdout.as_mut().expect("stdout")).expect("frame")
        else {
            break json!({"op":"eof"});
        };
        if matches!(frame["op"].as_str(), Some("complete" | "failure")) {
            break frame;
        }
        if frame["op"] == "conflict_prompt" {
            assert_eq!(
                frame["conflict_prompt"]["display"]["incoming_member"], "a/one",
                "exact protected incoming member"
            );
            assert_eq!(
                frame["conflict_prompt"]["display"]["target"], "a/one",
                "archive-internal existing object"
            );
            write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"conflict_reply","conflict_reply":{"ticket":frame["conflict_prompt"]["ticket"],"decision":"overwrite","apply_to_all":true}})).expect("original current ticket");
            continue;
        }
        if frame["op"] != "native_call" {
            continue;
        }
        let call = &frame["native"];
        let action = call["action"].as_str().expect("action");
        let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
        match action {
            "source_acquire" | "edit_planning" | "edit_reading" | "edit_reencoding"
            | "edit_verifying" | "creation_verifying" => {}
            "source_precommit" => {
                if matches!(fault, Fault::SourceChange) {
                    reply["success"] = json!(false);
                    reply["code"] = json!("source_changed");
                }
            }
            "source_stamp" => reply["read_data"] = json!("v1:fixture"),
            "source_roots" => {
                assert!(incoming);
                let file = json!({"id":source_id,"name":incoming_name,"directory":false,"size":3,"dos_modified":null});
                let entries = if matches!(recipe, "tree" | "conflict") {
                    json!([{"id":directory_id,"name":if recipe=="conflict"{"a"}else{"tree"},"directory":true,"size":0,"dos_modified":null}])
                } else {
                    json!([file])
                };
                reply["read_data"] = json!(entries.to_string());
            }
            "source_next" => {
                let child = if call["token"] == json!(directory_id) {
                    match call["offset"].as_u64().expect("offset") {
                        0 => {
                            json!({"id":source_id,"name":if recipe=="conflict"{"one"}else{"file"},"directory":false,"size":3,"dos_modified":null})
                        }
                        1 if recipe == "tree" => {
                            json!({"id":empty_id,"name":"empty","directory":true,"size":0,"dos_modified":null})
                        }
                        2 if recipe == "tree" => {
                            json!({"id":zero_id,"name":"zero","directory":false,"size":0,"dos_modified":null})
                        }
                        _ => Value::Null,
                    }
                } else {
                    Value::Null
                };
                reply["read_data"] = json!(child.to_string());
            }
            "source_open" => opened += 1,
            "source_close" => closed += 1,
            "source_check" => {}
            "source_read" => {
                let offset = call["offset"].as_u64().expect("offset") as usize;
                let bytes = if call["token"] == json!(zero_id) {
                    &b""[..]
                } else {
                    &b"new"[..]
                };
                let bytes = if matches!(fault, Fault::SourceEof) {
                    Vec::new()
                } else if matches!(fault, Fault::SourceExtra) && offset == 3 {
                    vec![1]
                } else {
                    bytes.get(offset..).unwrap_or_default().to_vec()
                };
                reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            }
            "begin" => {
                begins += 1;
                reply["token"] = json!(token);
            }
            "write" => {
                if matches!(fault, Fault::Write) {
                    reply["success"] = json!(false);
                    reply["code"] = json!("output_write_failed");
                } else {
                    let bytes: Vec<u8> =
                        serde_json::from_value(call["data"].clone()).expect("bytes");
                    partial.seek(SeekFrom::End(0)).expect("end");
                    partial.write_all(&bytes).expect("write");
                }
            }
            "read" => {
                let offset = call["offset"].as_u64().expect("offset");
                partial.seek(SeekFrom::Start(offset)).expect("seek");
                let mut bytes = vec![0; call["length"].as_u64().expect("length") as usize];
                let n = partial.read(&mut bytes).expect("read");
                bytes.truncate(n);
                if matches!(fault, Fault::Read) && !bytes.is_empty() {
                    bytes[0] ^= 1;
                }
                reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            }
            "edit_reads_complete" => {
                assert_eq!(
                    opened, closed,
                    "protected new source readers must be closed"
                );
                #[cfg(windows)]
                {
                    use std::os::windows::fs::OpenOptionsExt;
                    drop(
                        std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .share_mode(0)
                            .open(&path)
                            .expect("old readers closed"),
                    );
                }
                if matches!(fault, Fault::CancelBeforeHandoff) {
                    reply["success"] = json!(false);
                    reply["code"] = json!("cancelled");
                } else {
                    handed = true;
                }
            }
            "commit" => {
                assert!(handed);
                commits += 1;
                if matches!(fault, Fault::Commit5 | Fault::Commit64) {
                    reply["success"] = json!(false);
                    reply["code"] = json!("output_commit_failed");
                    reply["native_error_code"] = json!(if matches!(fault, Fault::Commit5) {
                        5
                    } else {
                        64
                    });
                }
            }
            "abort" => {}
            other => panic!("unexpected {other}"),
        }
        write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("reply");
    };
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("cleanup");
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    assert_eq!(opened, closed, "source closure including failure");
    assert_eq!(
        std::fs::read(&path).expect("retained"),
        original,
        "worker has no source mutation authority"
    );
    let members = if terminal["op"] == "complete" && commits == 1 {
        partial.rewind().expect("rewind");
        let candidate = root.path().join("oracle.tar");
        std::io::copy(
            &mut partial,
            &mut File::create(&candidate).expect("oracle copy"),
        )
        .expect("copy");
        let oracle = Command::new("python")
            .args(["-c", ORACLE])
            .arg(candidate)
            .output()
            .expect("independent oracle");
        assert!(
            oracle.status.success(),
            "{}",
            String::from_utf8_lossy(&oracle.stderr)
        );
        serde_json::from_slice(&oracle.stdout).expect("oracle JSON")
    } else {
        Value::Null
    };
    Outcome {
        terminal,
        members,
        commits,
        begins,
        handed,
    }
}
fn edit(action: &str, ordinals: Value, value: Value) -> Value {
    json!({"action":action,"ordinals":ordinals,"value":value})
}
fn success(result: &Outcome) {
    assert_eq!(result.terminal["op"], "complete", "{}", result.terminal);
    assert_eq!(result.commits, 1);
    assert!(result.handed);
}
#[test]
fn tar_edit_delete_overlap_subtree_boundary_and_legal_empty_oracle() {
    for format in ["Tar", "TarGz", "TarXz"] {
        let r = run(
            format,
            "ordinary",
            edit("delete", json!([0, 1, 2]), Value::Null),
            false,
            Fault::None,
            None,
            None,
        );
        success(&r);
        assert_eq!(r.members.as_object().expect("members").len(), 3);
        assert!(r.members.get("ab/keep").is_some());
        assert!(r.members.get("a/one").is_none());
        let r = run(
            format,
            "ordinary",
            edit("delete", json!([0, 1, 2, 3, 4, 5, 6]), Value::Null),
            false,
            Fault::None,
            None,
            None,
        );
        success(&r);
        assert_eq!(r.members, json!({}));
    }
}
#[test]
fn tar_edit_explicit_implicit_empty_directory_rename_and_unicode() {
    for (ordinals, prefix, value) in [
        (json!([0]), None, "改名/"),
        (json!([4]), Some("ab/"), "implicit/"),
        (json!([5]), None, "empty-new/"),
    ] {
        let mut e = edit("rename", ordinals, json!(value));
        if let Some(prefix) = prefix {
            e["source_prefix"] = json!(prefix);
        }
        let r = run("Tar", "ordinary", e, false, Fault::None, None, None);
        success(&r);
        assert!(
            r.members
                .get(if value == "改名/" {
                    "改名/one"
                } else if value == "implicit/" {
                    "implicit/keep"
                } else {
                    "empty-new"
                })
                .is_some()
        );
        assert_eq!(r.members["zero"]["content"], "");
    }
}
#[test]
fn tar_edit_add_tree_empty_file_directory_new_directory_and_replace() {
    for format in ["Tar", "TarGz", "TarXz"] {
        let r = run(
            format,
            "tree",
            edit("add", json!([]), Value::Null),
            true,
            Fault::None,
            None,
            None,
        );
        success(&r);
        assert_eq!(r.members["tree/file"]["content"], "new");
        assert_eq!(r.members["tree/zero"]["content"], "");
        assert_eq!(r.members["tree/empty"]["content"], Value::Null);
        let r = run(
            format,
            "ordinary",
            edit("replace_content", json!([1]), Value::Null),
            true,
            Fault::None,
            None,
            None,
        );
        success(&r);
        assert_eq!(r.members["a/one"]["content"], "new");
        assert_eq!(r.members["a/one"]["uid"], 123);
        assert_eq!(r.members["a/one"]["mode"], 0o640);
        let r = run(
            format,
            "ordinary",
            edit("new_directory", json!([]), json!("新增/")),
            false,
            Fault::None,
            None,
            None,
        );
        success(&r);
        assert_eq!(r.members["新增"]["content"], Value::Null);
    }
}
#[test]
fn tar_edit_pax_precision_identity_and_gnu_long_path_oracle() {
    let r = run(
        "Tar",
        "pax",
        edit("rename", json!([0]), json!("new")),
        false,
        Fault::None,
        None,
        None,
    );
    success(&r);
    let h = &r.members["new"];
    assert_eq!(h["content"], "data:old");
    assert_eq!(h["pax"]["mtime"], "1700000000.123456789");
    assert_eq!(h["pax"]["atime"], "1700000001.987654321");
    assert_eq!(h["pax"]["ctime"], "-1.25");
    assert_eq!(h["uid"], 12345678);
    assert_eq!(h["gid"], 87654321);
    assert_eq!(h["uname"], "用户");
    assert_eq!(h["gname"], "组");
    let name = format!("new/{}end", "segment/".repeat(20));
    let r = run(
        "Tar",
        "gnu",
        edit("rename", json!([0]), json!(name)),
        false,
        Fault::None,
        None,
        None,
    );
    success(&r);
    assert!(r.members.get(&name).is_some());
}
#[test]
fn tar_edit_unsafe_ambiguous_lossy_and_outer_shapes_refuse_before_native_candidate() {
    for recipe in [
        "duplicate",
        "case",
        "prefix",
        "unsafe",
        "link",
        "unknown",
        "global",
        "tail",
    ] {
        let r = run(
            "Tar",
            recipe,
            edit("rename", json!([0]), json!("new")),
            false,
            Fault::None,
            None,
            None,
        );
        assert_eq!(r.terminal["op"], "failure", "{recipe}: {}", r.terminal);
        assert_eq!(r.begins, 0);
        assert_eq!(r.commits, 0);
    }
    for format in ["TarGz", "TarXz"] {
        for recipe in ["tail", "truncated", "multi"] {
            let r = run(
                format,
                recipe,
                edit("rename", json!([0]), json!("new/")),
                false,
                Fault::None,
                None,
                None,
            );
            assert_eq!(
                r.terminal["op"], "failure",
                "{format}/{recipe}: {}",
                r.terminal
            );
            assert_eq!(r.begins, 0);
            assert_eq!(r.commits, 0);
        }
    }
    for value in ["a/sub/new/", "ab/", "zero/"] {
        let r = run(
            "Tar",
            "ordinary",
            edit("rename", json!([0]), json!(value)),
            false,
            Fault::None,
            None,
            None,
        );
        assert_eq!(r.terminal["op"], "failure");
        assert_eq!(r.begins, 0);
    }
}
#[test]
fn tar_edit_conflict_overwrite_skip_autorename_are_one_transaction() {
    for (policy, name) in [("overwrite", "a/one"), ("auto_rename", "a/one (2)")] {
        let r = run(
            "Tar",
            "conflict",
            edit("add", json!([]), Value::Null),
            true,
            Fault::None,
            Some(policy),
            None,
        );
        success(&r);
        assert_eq!(r.members[name]["content"], "new");
    }
    let r = run(
        "Tar",
        "conflict",
        edit("add", json!([]), Value::Null),
        true,
        Fault::None,
        Some("skip"),
        None,
    );
    assert_eq!(r.terminal["op"], "complete");
    assert_eq!(r.commits, 0);
    assert_eq!(r.begins, 0);
}
#[test]
fn tar_edit_write_read_source_eof_extra_cancel_commit_failures_keep_source() {
    for fault in [
        Fault::Write,
        Fault::Read,
        Fault::SourceChange,
        Fault::CancelBeforeHandoff,
        Fault::Commit5,
        Fault::Commit64,
    ] {
        let r = run(
            "Tar",
            "ordinary",
            edit("rename", json!([1]), json!("renamed")),
            false,
            fault,
            None,
            None,
        );
        assert_eq!(r.terminal["op"], "failure", "{}", r.terminal);
        if !matches!(fault, Fault::Commit5 | Fault::Commit64) {
            assert_eq!(r.commits, 0);
        }
    }
    for fault in [Fault::SourceEof, Fault::SourceExtra] {
        let r = run(
            "Tar",
            "ordinary",
            edit("replace_content", json!([1]), Value::Null),
            true,
            fault,
            None,
            None,
        );
        assert_eq!(r.terminal["op"], "failure");
        assert_eq!(r.begins, 0);
        assert_eq!(r.commits, 0);
    }
    let r = run(
        "Tar",
        "ordinary",
        edit("rename", json!([1]), json!("new")),
        false,
        Fault::None,
        None,
        Some(1024),
    );
    assert_eq!(r.terminal["op"], "failure");
    assert_eq!(r.begins, 0);
}

#[test]
fn step4_tar_edit_ask_identifies_archive_objects_before_single_candidate() {
    for format in ["Tar", "TarGz", "TarXz"] {
        let result = run(
            format,
            "conflict",
            edit("add", json!([]), Value::Null),
            true,
            Fault::None,
            Some("ask"),
            None,
        );
        success(&result);
        assert_eq!(result.members["a/one"]["content"], "new");
    }
}
