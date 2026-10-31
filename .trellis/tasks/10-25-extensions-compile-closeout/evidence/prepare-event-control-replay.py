from pathlib import Path
import hashlib,json
root=Path.cwd();out=Path(__file__).parent
mapping=root/'rust/apps/nxtools-desktop/src/extensions/event_history.rs'
source=root/'rust/apps/nxtools-desktop/src/extensions/committed_events.rs'
t=source.read_text(encoding='utf-8');m=mapping.read_text(encoding='utf-8')
owner=t[t.index('#[derive(Clone)]\npub(crate) struct EventControl'):t.index('/// Bounded transition memory')]
test=t[t.index('#[cfg(test)]\nmod compile_closeout_tests'):]
imports='use nxtools_extensions::{Causality, EventGuard};\nuse std::{collections::VecDeque, sync::{Arc, Mutex}, time::Instant};\nuse uuid::Uuid;\n'
(out/'isolated-committed-events.rs').write_text(imports+owner+test,encoding='utf-8')
(out/'isolated-current-event-history.rs').write_text(m,encoding='utf-8')
needle='                | RuntimeError::UnsupportedTransport\n';assert m.count(needle)==1
mutant=m.replace(needle,'').replace('            Err(RuntimeError::QueueFull) => Self::Busy,','            Err(RuntimeError::QueueFull) => Self::Busy,\n            Err(RuntimeError::UnsupportedTransport) => Self::Succeeded,')
assert mutant!=m
(out/'isolated-mutant-event-history.rs').write_text(mutant,encoding='utf-8')
for variant in ['current','mutant']:
    history=(out/('isolated-'+variant+'-event-history.rs')).as_posix()
    events=(out/'isolated-committed-events.rs').as_posix()
    (out/('event-control-'+variant+'.rs')).write_text('mod extensions {\n#[path="'+history+'"]\npub(crate) mod event_history;\n#[path="'+events+'"]\npub(crate) mod committed_events;\n}\n',encoding='utf-8')
facts={'production_owner_and_new_test_sha256':hashlib.sha256((owner+test).encode()).hexdigest(),'mapping_snapshot_sha256':hashlib.sha256(m.encode()).hexdigest(),'mutation':'scratch only: UnsupportedTransport -> Succeeded, keeping remaining match exhaustive','scope':'exact production EventControl and appended regression with real crate dependencies; isolated owner test, not full Desktop compilation'}
(out/'event-control-replay-sources.json').write_text(json.dumps(facts,indent=2)+'\n',encoding='utf-8');print(json.dumps(facts,indent=2))
