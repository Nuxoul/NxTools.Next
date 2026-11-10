//! Single-volume 7z edits over the original worker/native source and publication rendezvous.
use crate::{
    ArchiveError, ArchiveProgress, OperationControl, ResourceLimits, Result,
    creation::{CreationOptions, CreationPlan, CreationSource, IncomingPlan},
    interaction::InteractionPort,
    output::ArchiveOutput,
    sevenz_edit_plan::{self, Planned},
    worker::WorkerRequest,
    worker_conflict::{
        ConflictPolicy,
        add::{self, ExistingMember},
    },
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::{Reader, Sources},
    worker_tree_io::{Spool, io_error, stage_and_readback},
};
use nxtools_protocol::archive_job::ArchiveJobModifyAction as Action;
use sevenz_rust2::{ArchiveEntry, ArchiveReader, EncoderMethod, Password};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, Write},
    time::UNIX_EPOCH,
};

fn typed(error: sevenz_rust2::Error) -> ArchiveError {
    crate::worker_codecs::sevenz_error(error)
}
fn codec(error: ArchiveError) -> sevenz_rust2::Error {
    sevenz_rust2::Error::from(std::io::Error::other(error))
}
fn stamp(path: &str, control: &OperationControl) -> Result<([u8; 32], u64, u64)> {
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        control.checkpoint()?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ArchiveError::SourceChanged);
    }
    Ok((
        hash.finalize().into(),
        after.len(),
        u64::try_from(
            after
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ArchiveError::SourceChanged)?
                .as_nanos(),
        )
        .map_err(|_| ArchiveError::SourceChanged)?,
    ))
}

pub(crate) fn qualify(
    archive: &sevenz_rust2::Archive,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<bool> {
    crate::worker_codecs::sevenz_budget::blocks(archive, limits)?;
    let plain: Vec<_> = archive
        .files
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, entry)| Planned {
            entry,
            origin: Some(i),
            added: None,
        })
        .collect();
    sevenz_edit_plan::validate(&plain, limits, control)?;
    if archive.files.iter().any(|entry| {
        entry.is_anti_item
            || entry.has_windows_attributes
                && (entry.windows_attributes & 0x400 != 0
                    || entry.windows_attributes & 0x8000_0000 != 0
                        && ((entry.windows_attributes >> 16) & 0xf000)
                            != if entry.is_directory { 0x4000 } else { 0x8000 })
    }) {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let mut mode = None;
    for block in &archive.blocks {
        let encrypted = block
            .coders
            .iter()
            .any(|coder| coder.encoder_method_id() == EncoderMethod::ID_AES256_SHA256);
        if mode.is_some_and(|old| old != encrypted) {
            return Err(ArchiveError::UnsupportedEncryption);
        }
        mode = Some(encrypted);
    }
    Ok(mode.unwrap_or(false))
}
struct Payload<'a> {
    inner: &'a mut dyn Read,
    control: &'a OperationControl,
    maximum: u64,
    total: &'a Cell<u64>,
    size: u64,
    hash: Sha256,
    failure: &'a Cell<Option<ArchiveError>>,
}
impl Read for Payload<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let result = (|| {
            if let Some(error) = self.failure.get() {
                return Err(error);
            }
            self.control.checkpoint()?;
            let n = self.inner.read(buffer)?;
            self.size = self
                .size
                .checked_add(n as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            let total = self
                .total
                .get()
                .checked_add(n as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if self.size > self.maximum || total > self.maximum {
                return Err(ArchiveError::ResourceLimit);
            }
            self.total.set(total);
            self.hash.update(&buffer[..n]);
            self.control.publish(ArchiveProgress {
                bytes: total,
                files: 0,
                total_bytes: None,
            });
            Ok(n)
        })();
        result.map_err(|error| {
            if self.failure.get().is_none() {
                self.failure.set(Some(error));
            }
            std::io::Error::other(error)
        })
    }
}
fn encode<R: Read, W: Write + Seek>(
    writer: &mut sevenz_rust2::ArchiveWriter<W>,
    entry: &ArchiveEntry,
    reader: R,
) -> Result<()> {
    let mut entry = entry.clone();
    entry.has_crc = false;
    entry.crc = 0;
    entry.compressed_crc = 0;
    entry.compressed_size = 0;
    if entry.is_directory || entry.size == 0 {
        writer
            .push_archive_entry::<std::io::Empty>(entry, None)
            .map_err(typed)?;
    } else {
        writer
            .push_archive_entry(entry, Some(reader))
            .map_err(typed)?;
    }
    Ok(())
}
fn equal_metadata(a: &ArchiveEntry, b: &ArchiveEntry) -> bool {
    a.name == b.name
        && a.is_directory == b.is_directory
        && a.size == b.size
        && a.has_creation_date == b.has_creation_date
        && (!a.has_creation_date || a.creation_date == b.creation_date)
        && a.has_access_date == b.has_access_date
        && (!a.has_access_date || a.access_date == b.access_date)
        && a.has_last_modified_date == b.has_last_modified_date
        && (!a.has_last_modified_date || a.last_modified_date == b.last_modified_date)
        && a.has_windows_attributes == b.has_windows_attributes
        && (!a.has_windows_attributes || a.windows_attributes == b.windows_attributes)
}
fn verify(
    mut source: File,
    plan: &[Planned],
    hashes: &HashMap<String, [u8; 32]>,
    password: Password,
    encryption: (bool, bool),
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    let (header_encrypted, encrypted) = encryption;
    let facts = crate::sevenz_edit_header::inspect(&mut source, limits, control, &password)?;
    if facts.encrypted != header_encrypted {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut reader = ArchiveReader::new(source, password).map_err(typed)?;
    reader.set_thread_count(1);
    if reader.archive().files.len() != plan.len() || reader.archive().is_solid {
        return Err(ArchiveError::CorruptArchive);
    }
    qualify(reader.archive(), limits, control)?;
    for item in plan {
        let actual = reader
            .archive()
            .files
            .iter()
            .find(|entry| entry.name == item.entry.name)
            .ok_or(ArchiveError::CorruptArchive)?;
        if !equal_metadata(&item.entry, actual) {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    for block in &reader.archive().blocks {
        if block
            .coders
            .iter()
            .any(|coder| coder.encoder_method_id() == EncoderMethod::ID_AES256_SHA256)
            != encrypted
        {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    let failure = Cell::new(None);
    let total = Cell::new(0);
    let mut seen = HashSet::new();
    reader
        .for_each_entries(|entry, r| {
            let mut payload = Payload {
                inner: r,
                control,
                maximum: limits.max_output_bytes,
                total: &total,
                size: 0,
                hash: Sha256::new(),
                failure: &failure,
            };
            std::io::copy(&mut payload, &mut std::io::sink()).map_err(sevenz_rust2::Error::from)?;
            if payload.size != entry.size
                || hashes.get(&entry.name) != Some(&<[u8; 32]>::from(payload.hash.finalize()))
                || !seen.insert(entry.name.clone())
            {
                return Err(codec(ArchiveError::CorruptArchive));
            }
            Ok(true)
        })
        .map_err(|error| failure.get().unwrap_or_else(|| typed(error)))?;
    if seen.len() != plan.len() {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(())
}

pub(crate) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    interaction: &InteractionPort,
) -> Result<(u64, u64)> {
    let intent = request
        .job_modify
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    if intent.browser.format != "SevenZ" || request.volume_paths.len() > 1 {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let limits: ResourceLimits = request.limits.into();
    let original = stamp(&request.path, control)?;
    if original
        != (
            intent.browser.source.digest,
            intent.browser.source.size,
            intent.browser.source.modified_unix_ns,
        )
    {
        return Err(ArchiveError::SourceChanged);
    }
    let mut native = PrivateOutput::new(output, control, replies);
    native.source_checkpoint("edit_planning")?;
    native.edit_stage("edit_planning")?;
    if native.source_identity()? != intent.browser.source.native {
        return Err(ArchiveError::SourceChanged);
    }
    let password = request
        .password
        .as_deref()
        .map(Password::from)
        .unwrap_or_default();
    let mut source = File::open(&request.path)?;
    native.edit_stage("edit_reading")?;
    let facts = crate::sevenz_edit_header::inspect(&mut source, &limits, control, &password)?;
    let mut reader = ArchiveReader::new(source, password.clone()).map_err(typed)?;
    reader.set_thread_count(1);
    let encrypted = qualify(reader.archive(), &limits, control)?;
    if encrypted && password.is_empty() {
        return Err(ArchiveError::PasswordRequired);
    }
    if facts.encrypted && !encrypted && !reader.archive().files.iter().all(|e| !e.has_stream) {
        return Err(ArchiveError::UnsupportedEncryption);
    }
    // Decoder and encoder are simultaneously alive. Reserve both before encoder construction.
    let dictionary = lzma_rust2::LzmaOptions::with_preset(6).dict_size;
    let encoder = u64::from(dictionary) * 16 + (32 << 20);
    if encoder
        .checked_add(64 << 20)
        .is_none_or(|v| v > limits.max_working_bytes)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    let reduced = ResourceLimits {
        max_working_bytes: limits.max_working_bytes - encoder,
        ..limits
    };
    crate::worker_codecs::sevenz_budget::blocks(reader.archive(), &reduced)?;
    let mut plan = sevenz_edit_plan::build(&reader.archive().files, intent, &limits, control)?;
    let failure = Cell::new(None);
    let shared = RefCell::new(&mut native);
    let mut sources = Sources::new(&shared);
    let addition_plan = if intent.action == Action::Add {
        Some(IncomingPlan::build(&mut sources, &limits, control)?.entries)
    } else if intent.action == Action::ReplaceContent {
        Some(
            CreationPlan::build(
                &mut sources,
                &CreationOptions {
                    strip_single_root: false,
                    exclude_patterns: vec![],
                },
                &limits,
                control,
            )?
            .entries,
        )
    } else {
        None
    };
    let mut skipped = 0u64;
    let mut add_result = None;
    if let Some(additions) = &addition_plan {
        if intent.action == Action::ReplaceContent {
            if additions.len() != 1 || additions[0].is_directory() {
                return Err(ArchiveError::UnsupportedOperation);
            }
            for item in &mut plan {
                if item.added == Some(0) {
                    item.entry.size = additions[0].size();
                }
            }
        } else {
            let (selected, skipped_files) = add::resolve(
                additions,
                plan.iter().map(|item| ExistingMember {
                    name: &item.entry.name,
                    directory: item.entry.is_directory,
                    size: Some(item.entry.size),
                }),
                intent.value.as_deref().unwrap_or(""),
                Some(request.conflict_policy.unwrap_or(ConflictPolicy::Ask)),
                &mut shared.borrow_mut(),
                interaction,
                &limits,
            )?;
            if !selected.is_empty() {
                add_result = Some(add::display_result(
                    additions,
                    intent.value.as_deref().unwrap_or(""),
                    &selected,
                ));
            }
            skipped = skipped_files;
            for selected in selected {
                let item = &additions[selected.input];
                if let Some(old) = selected.replaces {
                    plan[old].entry.size = item.size();
                    plan[old].origin = None;
                    plan[old].added = Some(selected.input);
                } else {
                    let name = selected.name.trim_end_matches('/');
                    let mut entry = if item.is_directory() {
                        ArchiveEntry::new_directory(name)
                    } else {
                        ArchiveEntry::new_file(name)
                    };
                    entry.size = item.size();
                    plan.push(Planned {
                        entry,
                        origin: None,
                        added: Some(selected.input),
                    });
                }
            }
        }
    }
    sevenz_edit_plan::validate(&plan, &limits, control)?;
    if intent.action == Action::Add && plan.iter().all(|item| item.added.is_none()) {
        sources.revalidate()?;
        if stamp(&request.path, control)? != original {
            return Err(ArchiveError::SourceChanged);
        }
        shared.borrow_mut().member_summary(skipped, false)?;
        return Ok((0, 0));
    }
    if encrypted && !facts.encrypted && plan.iter().all(|item| item.entry.size == 0) {
        return Err(ArchiveError::UnsupportedEncryption);
    }
    let metadata = plan.iter().try_fold(4u64 << 20, |sum, item| {
        sum.checked_add(8192 + item.entry.name.len() as u64 * 128)
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    if metadata > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let sources = RefCell::new(sources);
    let digests = RefCell::new(vec![None; addition_plan.as_ref().map_or(0, Vec::len)]);
    shared.borrow_mut().source_checkpoint("edit_reencoding")?;
    shared.borrow_mut().edit_stage("edit_reencoding")?;
    let spool = Spool::new(limits.max_output_bytes, control, &failure)?;
    let initialized = sevenz_rust2::ArchiveWriter::new(spool).map_err(typed);
    if let Some(error) = failure.get() {
        return Err(error);
    }
    let mut writer = initialized?;
    writer.set_encrypt_header(facts.encrypted);
    let mut methods = Vec::new();
    if encrypted || facts.encrypted {
        methods
            .push(sevenz_rust2::encoder_options::AesEncoderOptions::new(password.clone()).into());
    }
    methods.push(sevenz_rust2::encoder_options::Lzma2Options::from_level(1).into());
    writer.set_content_methods(methods);
    let mut hashes = HashMap::new();
    let total = Cell::new(0);
    let mut index_by_name = HashMap::new();
    for (i, entry) in reader.archive().files.iter().enumerate() {
        if index_by_name.insert(entry.name.clone(), i).is_some() {
            return Err(ArchiveError::OutputCollision);
        }
    }
    reader
        .for_each_entries(|entry, r| {
            if let Some(error) = failure.get() {
                return Err(codec(error));
            }
            let index = *index_by_name
                .get(&entry.name)
                .ok_or_else(|| codec(ArchiveError::SourceChanged))?;
            let mut payload = Payload {
                inner: r,
                control,
                maximum: limits.max_output_bytes,
                total: &total,
                size: 0,
                hash: Sha256::new(),
                failure: &failure,
            };
            let target = plan
                .iter()
                .find(|item| item.origin == Some(index) && item.added.is_none());
            if let Some(target) = target {
                encode(&mut writer, &target.entry, &mut payload).map_err(codec)?;
            }
            std::io::copy(&mut payload, &mut std::io::sink()).map_err(sevenz_rust2::Error::from)?;
            if payload.size != entry.size || payload.size > limits.max_entry_bytes {
                return Err(codec(ArchiveError::CorruptArchive));
            }
            if let Some(target) = target {
                hashes.insert(target.entry.name.clone(), payload.hash.finalize().into());
            }
            Ok(true)
        })
        .map_err(|error| failure.get().unwrap_or_else(|| typed(error)))?;
    drop(reader);
    for item in &plan {
        if let Some(index) = item.added {
            let additions = addition_plan.as_ref().ok_or(ArchiveError::SourceChanged)?;
            if item.entry.is_directory {
                encode(&mut writer, &item.entry, std::io::empty())?;
                hashes.insert(item.entry.name.clone(), Sha256::digest([]).into());
                continue;
            }
            let mut source = Reader::new(
                &sources,
                &additions[index],
                control,
                &failure,
                &digests,
                index,
            );
            let encoded = encode(&mut writer, &item.entry, &mut source);
            if let Some(error) = failure.get() {
                return Err(error);
            }
            encoded?;
            let mut sink = [0; 1];
            if source.read(&mut sink).map_err(io_error)? != 0 {
                return Err(ArchiveError::SourceChanged);
            }
            hashes.insert(
                item.entry.name.clone(),
                digests.borrow()[index].ok_or(ArchiveError::SourceChanged)?,
            );
        } else if item.origin.is_none() {
            encode(&mut writer, &item.entry, std::io::empty())?;
            hashes.insert(item.entry.name.clone(), Sha256::digest([]).into());
        }
    }
    let finished = writer.finish().map_err(io_error);
    if let Some(error) = failure.get() {
        return Err(error);
    }
    let mut encoded = finished?;
    shared.borrow_mut().source_checkpoint("edit_verifying")?;
    shared.borrow_mut().edit_stage("edit_verifying")?;
    encoded.file.rewind()?;
    let mut verify_source = encoded.file.try_clone()?;
    verify_source.rewind()?;
    verify(
        verify_source,
        &plan,
        &hashes,
        password.clone(),
        (facts.encrypted, encrypted),
        &limits,
        control,
    )?;
    if stamp(&request.path, control)? != original {
        return Err(ArchiveError::SourceChanged);
    }
    sources.borrow_mut().revalidate()?;
    let name = std::path::Path::new(&request.path)
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or(ArchiveError::UnsafePath)?;
    let token = shared.borrow_mut().begin_file(name)?;
    let result = (|| {
        let readback = stage_and_readback(
            &mut encoded.file,
            &mut shared.borrow_mut(),
            token,
            limits.max_output_bytes,
            control,
        )?;
        verify(
            readback,
            &plan,
            &hashes,
            password,
            (facts.encrypted, encrypted),
            &limits,
            control,
        )?;
        let bytes = encoded.file.metadata()?.len();
        drop(encoded);
        shared.borrow_mut().source_checkpoint("source_precommit")?;
        if stamp(&request.path, control)? != original {
            return Err(ArchiveError::SourceChanged);
        }
        sources.borrow_mut().revalidate()?;
        control.checkpoint()?;
        shared.borrow_mut().edit_stage("edit_committing")?;
        shared
            .borrow_mut()
            .call("edit_reads_complete", None, Some(token), &[])?;
        shared.borrow_mut().commit_file(token)?;
        Ok((1, bytes))
    })();
    if result.is_err() {
        shared.borrow_mut().abort_file(token);
    }
    if result.is_ok() {
        shared.borrow_mut().member_summary(skipped, skipped == 0)?;
        if let Some(add_result) = add_result {
            shared.borrow_mut().add_result(add_result)?;
        }
    }
    result
}
