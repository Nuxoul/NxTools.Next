use super::*;
struct Storage {
    fail_write: bool,
    fail_flush: bool,
    written: usize,
    flushed: bool,
}
impl Write for Storage {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self.fail_write {
            return Err(std::io::Error::other(ArchiveError::IoFailure));
        }
        self.written += b.len();
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.flushed = true;
        if self.fail_flush {
            Err(std::io::Error::other(ArchiveError::IoFailure))
        } else {
            Ok(())
        }
    }
}
#[test]
fn tar_edit_plain_finish_and_flush_errors_are_terminal() {
    for fail_write in [true, false] {
        let mut storage = Storage {
            fail_write,
            fail_flush: !fail_write,
            written: 0,
            flushed: false,
        };
        assert_eq!(finish(&mut storage), Err(ArchiveError::IoFailure));
        assert_eq!(storage.flushed, !fail_write);
        assert_eq!(storage.written, if fail_write { 0 } else { 1024 });
    }
}

#[test]
fn tar_edit_prefix_cannot_promote_file_to_directory_and_implicit_case_is_ambiguous() {
    let control = OperationControl::default();
    let limits = ResourceLimits::default();
    let old = vec![Member::new("a".into(), false, 0).expect("member")];
    let intent:nxtools_protocol::archive_job::ArchiveJobModify=serde_json::from_value(serde_json::json!({"browser":{"snapshot_id":uuid::Uuid::new_v4(),"format":"Tar","comment":"","source":{"native":"v1:fixture","digest":vec![0;32],"size":0,"modified_unix_ns":0}},"action":"rename","ordinals":[0],"source_prefix":"a/","value":"renamed/"})).expect("intent");
    assert!(matches!(
        plan(&old, &intent, &limits, &control),
        Err(ArchiveError::SourceChanged)
    ));
    let old = vec![
        Member::new("a/one".into(), false, 0).expect("member"),
        Member::new("A/two".into(), false, 0).expect("member"),
    ];
    assert_eq!(
        validate(&old, &limits, &control),
        Err(ArchiveError::OutputCollision)
    );
}

#[test]
fn opt05_tar_edit_delete_expands_once() {
    let (snapshot, ids) = crate::opt05_edit_selection_fixture::large();
    let old = snapshot
        .entries
        .iter()
        .enumerate()
        .map(|(ordinal, entry)| {
            let mut member = Member::new(
                entry.display_path.clone(),
                entry.kind == crate::EntryKind::Directory,
                0,
            )
            .expect("original member");
            member.origin = Some(ordinal);
            member
        })
        .collect::<Vec<_>>();
    let intent = crate::opt05_edit_selection_fixture::delete_intent(
        ids.iter().map(|id| id.ordinal).collect(),
    );
    let control = OperationControl::default();
    let started = std::time::Instant::now();
    let result = plan(&old, &intent, &ResourceLimits::default(), &control)
        .expect("original TAR plan boundary");
    assert!(result.is_empty());
    let work = control.work_observation().selection;
    eprintln!(
        "OPT05_EDIT_SELECTION {}",
        serde_json::json!({
            "boundary":"tar_delete_plan", "elapsed_us":started.elapsed().as_micros(), "work":work,
        })
    );
    assert!(
        work.catalog_visits <= 65_536
            && work.path_normalizations + work.root_normalizations <= 65_536
            && work.prefix_probes <= 65_536 * 16 + 4_096 * 64,
        "TAR delete expansion is one original catalog pass: {work:?}"
    );
}
