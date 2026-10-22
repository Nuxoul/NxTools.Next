//! Independent metadata credit. Never a native-output call/token acknowledgement.
use crate::{
    ArchiveEntry, ArchiveError, NameEncoding, NameProvenance, OperationControl, Result,
    worker::{WorkerEntry, WorkerFrame, WorkerRequest, write_frame},
};
use std::{
    io::Write,
    sync::{Arc, Condvar, Mutex},
};
use uuid::Uuid;
struct State {
    pending: Option<u64>,
    acknowledged: bool,
    cancelled: bool,
    entries: u64,
    batches: u64,
    upgrade: Option<crate::worker::ReadUpgrade>,
    upgrade_received: bool,
    upgrade_sealed: bool,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
}
#[derive(Clone)]
pub(crate) struct MetadataCredit(Arc<Shared>);
impl MetadataCredit {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                pending: None,
                acknowledged: false,
                cancelled: false,
                entries: 0,
                batches: 0,
                upgrade: None,
                upgrade_received: false,
                upgrade_sealed: false,
            }),
            ready: Condvar::new(),
        }))
    }
    pub(crate) fn upgrade(&self, value: crate::worker::ReadUpgrade) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.upgrade_received
            || state.upgrade_sealed
            || state.pending.is_some()
            || state.batches != 0
            || state.cancelled
        {
            return false;
        }
        state.upgrade = Some(value);
        state.upgrade_received = true;
        true
    }
    pub(crate) fn take_upgrade(&self) -> Option<crate::worker::ReadUpgrade> {
        let mut state = self.0.state.lock().ok()?;
        state.upgrade_sealed = true;
        state.upgrade.take()
    }
    pub(crate) fn acknowledge(&self, sequence: u64) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.pending != Some(sequence) || state.acknowledged || state.cancelled {
            return false;
        }
        state.acknowledged = true;
        self.0.ready.notify_all();
        true
    }
    fn install(&self, sequence: u64) -> Result<()> {
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        if state.cancelled {
            return Err(ArchiveError::Cancelled);
        }
        if state.pending.is_some() {
            return Err(ArchiveError::Busy);
        }
        state.pending = Some(sequence);
        state.acknowledged = false;
        Ok(())
    }
    fn wait(&self, control: &OperationControl) -> Result<()> {
        let weak = Arc::downgrade(&self.0);
        let _registration = control.on_cancel(Arc::new(move || {
            if let Some(shared) = weak.upgrade()
                && let Ok(mut state) = shared.state.lock()
            {
                state.cancelled = true;
                shared.ready.notify_all();
            }
        }))?;
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        loop {
            if state.cancelled {
                state.pending = None;
                drop(state);
                control.observe_cancelled(crate::control::CancellationBoundary::MetadataCredit);
                return Err(ArchiveError::Cancelled);
            }
            if state.acknowledged {
                state.pending = None;
                state.acknowledged = false;
                return Ok(());
            }
            state = self
                .0
                .ready
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
        }
    }
    pub(crate) fn counts(&self) -> (u64, u64) {
        self.0
            .state
            .lock()
            .map(|state| (state.entries, state.batches))
            .unwrap_or_default()
    }
    fn record(&self, entries: usize) -> Result<()> {
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        state.entries += entries as u64;
        state.batches += 1;
        Ok(())
    }
}
fn codepage(encoding: NameEncoding) -> u32 {
    match encoding {
        NameEncoding::Auto => 437,
        NameEncoding::Utf8 => 65001,
        NameEncoding::Gbk => 936,
        NameEncoding::Big5 => 950,
        NameEncoding::ShiftJis => 932,
        NameEncoding::Korean => 949,
        NameEncoding::Cp437 => 437,
        NameEncoding::Windows1250 => 1250,
        NameEncoding::Windows1251 => 1251,
        NameEncoding::Windows1252 => 1252,
    }
}
fn entry(value: &ArchiveEntry, product: bool, browser: bool) -> WorkerEntry {
    let (provenance, cp) = match value.name_provenance() {
        NameProvenance::Utf8Flag => ("Utf8Flag", 65001),
        NameProvenance::UnicodeExtra => ("UnicodeExtra", 65001),
        NameProvenance::FormatUnicode => ("FormatUnicode", 65001),
        NameProvenance::UncertainCp437 => ("UncertainCp437", 437),
        NameProvenance::Explicit(encoding) => ("Explicit", codepage(encoding)),
        NameProvenance::Heuristic(encoding) => ("Heuristic", codepage(encoding)),
    };
    WorkerEntry {
        ordinal: value.id().ordinal,
        name: value.display_path().to_owned(),
        kind: format!("{:?}", value.kind()),
        size: value.size(),
        packed_size: value.compressed_size(),
        encrypted: Some(value.encrypted()),
        warning_codes: value.warnings().iter().map(|v| format!("{v:?}")).collect(),
        provenance: product.then(|| provenance.into()),
        codepage: product.then_some(cp),
        modified: browser.then_some(value.metadata().modified).flatten(),
        method: (browser && (value.zip.is_some() || value.method() != 0)).then_some(value.method()),
        crc32: browser.then_some(value.crc32()).flatten(),
        attributes: browser.then_some(value.metadata().attributes).flatten(),
    }
}
// C#'s default JSON encoder can expand non-ASCII and HTML-sensitive characters. Charge an
// upper bound over the ACTUAL serialized entry, not just UTF-8 name bytes or entry count.
fn receiver_json_bytes(bytes: &[u8]) -> Result<usize> {
    let text = std::str::from_utf8(bytes).map_err(|_| ArchiveError::IoFailure)?;
    Ok(text
        .chars()
        .map(|value| {
            if !value.is_ascii() {
                6 * value.len_utf16()
            } else if matches!(value, '\u{7f}' | '"' | '<' | '>' | '&' | '\'' | '+' | '`') {
                6
            } else {
                1
            }
        })
        .sum())
}
pub(crate) fn emit_entries<W: Write>(
    request: &WorkerRequest,
    entries: &[ArchiveEntry],
    snapshot: Uuid,
    format: &str,
    output: &mut W,
    control: &OperationControl,
    credit: &MetadataCredit,
) -> Result<()> {
    let product = request.read_contract.is_some();
    let large = request.read_contract == Some(2);
    let mut offset = 0;
    let mut sequence = 0;
    while offset < entries.len() {
        control.checkpoint()?;
        let mut frame = WorkerFrame::state(request.operation_id, "entry_batch");
        frame.format = Some(format.into());
        if product {
            frame.snapshot_id = Some(snapshot);
            frame.batch_sequence = Some(sequence);
        }
        let entry_limit = if large { 256 } else { 2 };
        // Reserve the actual frame envelope; charge each escaped serialized entry BEFORE adding.
        let mut bytes = serde_json::to_vec(&frame)
            .map_err(|_| ArchiveError::IoFailure)?
            .len()
            + 13;
        while offset + frame.entries.len() < entries.len() && frame.entries.len() < entry_limit {
            let value = entry(
                &entries[offset + frame.entries.len()],
                product,
                large
                    && request
                        .read_transport
                        .is_some_and(|transport| transport.browser_context),
            );
            let serialized = serde_json::to_vec(&value).map_err(|_| ArchiveError::IoFailure)?;
            let size = if large {
                receiver_json_bytes(&serialized)?
            } else {
                serialized.len()
            } + 1;
            if large && bytes + size > 256 << 10 {
                if frame.entries.is_empty() {
                    return Err(ArchiveError::ResourceLimit);
                }
                break;
            }
            bytes += size;
            frame.entries.push(value);
        }
        let count = frame.entries.len();
        if product {
            credit.install(sequence)?;
        }
        write_frame(output, &frame).map_err(|_| ArchiveError::IoFailure)?;
        if product {
            credit.wait(control)?;
            credit.record(count)?;
        }
        offset += count;
        sequence += 1;
    }
    Ok(())
}
