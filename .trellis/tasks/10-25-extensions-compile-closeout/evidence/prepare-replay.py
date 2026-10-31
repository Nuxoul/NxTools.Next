from pathlib import Path
import json,hashlib
root=Path.cwd();out=Path(__file__).parent
historical=root/'.trellis/tasks/10-25-rem-new-download/evidence/checkpoint-01/blocker-rust__apps__nxtools-desktop__src__extensions__event_history.rs.observed'
current=root/'rust/apps/nxtools-desktop/src/extensions/event_history.rs'
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(historical.read_bytes())=='d8a8fc08c6724aef546b13c00cbeb014a71c64daec699d13a5bf23c7b25bcd62'
for name,path in [('historical',historical),('current',current)]:
    snapshot=out/(name+'-event-history.rs');snapshot.write_bytes(path.read_bytes())
    # The real public RuntimeError/InvokeOutcome and observed production mapping are used.
    (out/(name+'-mapping-replay.rs')).write_text('#[path = "'+snapshot.name+'"]\nmod event_history;\n',encoding='utf-8')
(out/'current-mapping-test.rs').write_text('''#[path = "current-event-history.rs"]
mod event_history;
#[test]
fn actual_unsupported_transport_mapping_is_unavailable_not_success() {
    let observed = event_history::EventOutcome::from_result(
        &Err(nxtools_extensions::RuntimeError::UnsupportedTransport),
    );
    assert_eq!(observed, event_history::EventOutcome::Unavailable);
    assert!(!observed.success());
}
''',encoding='utf-8')
facts={'historical_sha256':sha(historical.read_bytes()),'current_snapshot_sha256':sha((out/'current-event-history.rs').read_bytes()),'scope':'scratch-only replay; real nxtools_extensions types, not a live-source rollback or full Desktop acceptance'}
(out/'replay-sources.json').write_text(json.dumps(facts,indent=2)+'\n',encoding='utf-8');print(json.dumps(facts,indent=2))
