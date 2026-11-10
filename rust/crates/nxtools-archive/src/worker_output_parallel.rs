//! Joined ordinary-output lanes share frames and decisions, never payload buffers.
use super::*;
use crate::worker_conflict::{ExtractionConflict, NativeTarget};
use std::io;

pub(super) trait DecisionGate: Sync {
    fn resolve(
        &self,
        native: &mut dyn NativeTarget,
        observed: NativeReply,
        path: &str,
        member: &str,
        incoming: Option<u64>,
    ) -> Result<Option<Uuid>>;
    fn confirm(
        &self,
        entry: &crate::ArchiveEntry,
        limits: &crate::ResourceLimits,
        control: &OperationControl,
        writer: &mut dyn Write,
    ) -> Result<()>;
    fn skipped(&self) -> u64;
}
struct Decisions<'a> {
    conflict: Option<ExtractionConflict<'a>>,
    risk: Option<crate::zip_risk::ZipRisk<'a>>,
}
impl DecisionGate for Mutex<Decisions<'_>> {
    fn resolve(
        &self,
        native: &mut dyn NativeTarget,
        observed: NativeReply,
        path: &str,
        member: &str,
        incoming: Option<u64>,
    ) -> Result<Option<Uuid>> {
        let mut decisions = self.lock().map_err(|_| ArchiveError::IoFailure)?;
        decisions
            .conflict
            .as_mut()
            .ok_or(ArchiveError::OutputCollision)?
            .resolve(native, observed, path, member, incoming)
    }
    fn confirm(
        &self,
        entry: &crate::ArchiveEntry,
        limits: &crate::ResourceLimits,
        control: &OperationControl,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let decisions = self.lock().map_err(|_| ArchiveError::IoFailure)?;
        match &decisions.risk {
            Some(risk) => risk.confirm(entry, limits, control, writer),
            None if crate::zip_risk::required(entry) => Err(ArchiveError::ResourceLimit),
            None => Ok(()),
        }
    }
    fn skipped(&self) -> u64 {
        self.lock().map_or(0, |decisions| {
            decisions
                .conflict
                .as_ref()
                .map_or(0, |conflict| conflict.skipped)
        })
    }
}
struct FrameState<'a, W> {
    writer: &'a mut W,
    owner: Option<Uuid>,
}
struct FrameSink<'a, W> {
    state: Mutex<FrameState<'a, W>>,
    ready: Condvar,
}
trait FrameTarget: Sync {
    fn write_frame_part(&self, owner: Uuid, bytes: &[u8]) -> io::Result<usize>;
    fn finish_frame(&self, owner: Uuid) -> io::Result<()>;
    fn release(&self, owner: Uuid);
}
impl<W: Write + Send> FrameTarget for FrameSink<'_, W> {
    fn write_frame_part(&self, owner: Uuid, bytes: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().map_err(|_| io::ErrorKind::Other)?;
        while state.owner.is_some_and(|current| current != owner) {
            state = self.ready.wait(state).map_err(|_| io::ErrorKind::Other)?;
        }
        state.owner = Some(owner);
        let result = state.writer.write(bytes);
        if result.is_err() {
            state.owner = None;
            self.ready.notify_all();
        }
        result
    }
    fn finish_frame(&self, owner: Uuid) -> io::Result<()> {
        let mut state = self.state.lock().map_err(|_| io::ErrorKind::Other)?;
        while state.owner.is_some_and(|current| current != owner) {
            state = self.ready.wait(state).map_err(|_| io::ErrorKind::Other)?;
        }
        let result = state.writer.flush();
        state.owner = None;
        self.ready.notify_all();
        result
    }
    fn release(&self, owner: Uuid) {
        if let Ok(mut state) = self.state.lock()
            && state.owner == Some(owner)
        {
            state.owner = None;
            self.ready.notify_all();
        }
    }
}
pub(super) struct FrameWriter<'a> {
    sink: &'a dyn FrameTarget,
    id: Uuid,
}
impl Write for FrameWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.sink.write_frame_part(self.id, bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.sink.finish_frame(self.id)
    }
}
impl Drop for FrameWriter<'_> {
    fn drop(&mut self) {
        self.sink.release(self.id);
    }
}
pub(super) enum Writer<'a, W: Write> {
    Direct(&'a mut W),
    Shared(FrameWriter<'a>),
}
#[cfg(test)]
impl<W: Write> Writer<'_, W> {
    pub(super) fn direct(&self) -> &W {
        match self {
            Self::Direct(writer) => writer,
            Self::Shared(_) => panic!("serial test peer"),
        }
    }
}
impl<W: Write> Write for Writer<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Direct(writer) => writer.write(bytes),
            Self::Shared(writer) => writer.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Direct(writer) => writer.flush(),
            Self::Shared(writer) => writer.flush(),
        }
    }
}
struct Factory<'a, 'd, W: Write> {
    sink: FrameSink<'a, W>,
    control: &'a OperationControl,
    replies: ReplyPort,
    decisions: Mutex<Decisions<'d>>,
    metadata: bool,
    display_prefix: Option<String>,
}
impl<W: Write + Send> crate::formats::zip::parallel::OutputFactory for Factory<'_, '_, W> {
    fn output(&self) -> Box<dyn ArchiveOutput + '_> {
        let writer: Writer<'_, W> = Writer::Shared(FrameWriter {
            sink: &self.sink,
            id: Uuid::new_v4(),
        });
        let mut output = PrivateOutput::from_writer(writer, self.control, self.replies.clone())
            .ordinary_extraction(true)
            .extraction_metadata(self.metadata);
        output.shared_decisions = Some(&self.decisions);
        output.display_prefix.clone_from(&self.display_prefix);
        output.parallel_output = true;
        Box::new(output)
    }
    fn fail(&self, error: ArchiveError) {
        self.replies.fail(error);
        self.control.cancel();
    }
}
impl<W: Write + Send> PrivateOutput<'_, W> {
    pub(crate) fn with_parallel<R>(
        &mut self,
        run: impl FnOnce(&dyn crate::formats::zip::parallel::OutputFactory) -> R,
    ) -> R {
        let Writer::Direct(writer) = &mut self.writer else {
            unreachable!("one joined parallel scope per ordinary output")
        };
        let factory = Factory {
            sink: FrameSink {
                state: Mutex::new(FrameState {
                    writer: &mut **writer,
                    owner: None,
                }),
                ready: Condvar::new(),
            },
            control: self.control,
            replies: self.replies.clone(),
            decisions: Mutex::new(Decisions {
                conflict: self.conflict.take(),
                risk: self.risk.take(),
            }),
            metadata: self.metadata_enabled,
            display_prefix: self.display_prefix.clone(),
        };
        let result = run(&factory);
        let decisions = factory
            .decisions
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.conflict = decisions.conflict;
        self.risk = decisions.risk;
        result
    }
}
