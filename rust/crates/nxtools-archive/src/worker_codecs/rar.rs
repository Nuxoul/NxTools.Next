//! Narrow UnRAR codec FFI, not a Windows API adapter. No output path is ever supplied.
//! Safety: boxed callback state and UTF16 archive path outlive the native handle;
//! callbacks lock shared state, never retain library buffers or unwind across C ABI.
#![allow(unsafe_code)]
use super::*;
use std::os::windows::ffi::OsStrExt;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::Mutex,
};
use unrar_sys as native;
use zeroize::Zeroize;

// Locked UnRAR rartypes.hpp INT64NDF, surfaced by dll.cpp for UnknownUnpSize.
const UNKNOWN_SIZE: u64 = 0x7fff_ffff_7fff_ffff;

// Exact locked vendor/unrar/dll.hpp4,78-117 ABI. upstream HeaderDataEx omits pack1.
#[repr(C, packed)]
struct PackedHeader {
    archive_name: [i8; 1024],
    archive_name_w: [u16; 1024],
    filename: [i8; 1024],
    filename_w: [u16; 1024],
    flags: u32,
    pack_size: u32,
    pack_size_high: u32,
    unp_size: u32,
    unp_size_high: u32,
    host_os: u32,
    file_crc: u32,
    file_time: u32,
    unp_ver: u32,
    method: u32,
    file_attr: u32,
    comment_buffer: *mut i8,
    comment_buffer_size: u32,
    comment_size: u32,
    comment_state: u32,
    dict_size: u32,
    hash_type: u32,
    hash: [i8; 32],
    redir_type: u32,
    redir_name: *mut u16,
    redir_name_size: u32,
    dir_target: u32,
    mtime_low: u32,
    mtime_high: u32,
    ctime_low: u32,
    ctime_high: u32,
    atime_low: u32,
    atime_high: u32,
    archive_name_ex: *mut u16,
    archive_name_ex_size: u32,
    filename_ex: *mut u16,
    filename_ex_size: u32,
    reserved: [u32; 982],
}
unsafe extern "system" {
    #[link_name = "RARReadHeaderEx"]
    fn read_header(handle: *const native::Handle, header: *mut PackedHeader) -> i32;
}
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::size_of::<PackedHeader>() == 10244);
    assert!(std::mem::offset_of!(PackedHeader, comment_buffer) == 6188);
    assert!(std::mem::offset_of!(PackedHeader, mtime_low) == 6268);
};

struct CallbackState<'a> {
    control: OperationControl,
    password: Option<Vec<u16>>,
    limits: ResourceLimits,
    entry_bytes: u64,
    total_bytes: u64,
    error: Option<ArchiveError>,
    selected_limit: Option<u64>,
    output: Option<&'a mut (dyn CodecOutput + Send)>,
    token: Option<uuid::Uuid>,
    pending: Vec<(uuid::Uuid, crate::metadata::OutputMetadata)>,
    decoding_checkpoint: bool,
    volume_names: Vec<Vec<u16>>,
    next_volume: usize,
    notifying_ansi: bool,
}
impl Drop for CallbackState<'_> {
    fn drop(&mut self) {
        if let Some(output) = &mut self.output {
            for (token, _) in self.pending.drain(..) {
                output.abort_file(token);
            }
        }
        if let Some(password) = &mut self.password {
            password.zeroize();
        }
    }
}
extern "C" fn callback(
    message: native::UINT,
    user: native::LPARAM,
    first: native::LPARAM,
    second: native::LPARAM,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if user == 0 {
            return -1;
        }
        // SAFETY: user points to the boxed Mutex owned by execute until after RARCloseArchive.
        let state = unsafe { &*(user as *const Mutex<CallbackState<'_>>) };
        let Ok(mut state) = state.lock() else {
            return -1;
        };
        if state.error.is_some() {
            return -1;
        }
        if let Err(error) = state.control.checkpoint() {
            state.error = Some(error);
            return -1;
        }
        match message {
            native::UCM_PROCESSDATA => {
                if !state.decoding_checkpoint {
                    state.decoding_checkpoint = true;
                    if let Some(output) = &mut state.output
                        && let Err(error) = output.source_checkpoint("source_decoding")
                    {
                        state.error = Some(error);
                        return -1;
                    }
                }
                let Ok(count) = u64::try_from(second) else {
                    state.error = Some(ArchiveError::CorruptArchive);
                    return -1;
                };
                let Some(entry) = state.entry_bytes.checked_add(count) else {
                    state.error = Some(ArchiveError::ResourceLimit);
                    return -1;
                };
                let Some(total) = state.total_bytes.checked_add(count) else {
                    state.error = Some(ArchiveError::ResourceLimit);
                    return -1;
                };
                state.entry_bytes = entry;
                state.total_bytes = total;
                if entry > state.limits.max_entry_bytes
                    || total > state.limits.max_output_bytes
                    || state
                        .selected_limit
                        .is_some_and(|limit| state.token.is_some() && entry > limit)
                {
                    state.error = Some(ArchiveError::ResourceLimit);
                    return -1;
                }
                if let Some(token) = state.token {
                    if count > 0 && first == 0 {
                        state.error = Some(ArchiveError::CorruptArchive);
                        return -1;
                    }
                    let Ok(count) = usize::try_from(count) else {
                        state.error = Some(ArchiveError::ResourceLimit);
                        return -1;
                    };
                    if let Some(output) = &mut state.output {
                        let mut offset = 0;
                        while offset < count {
                            let length = (count - offset).min(65536);
                            // SAFETY: UnRAR owns first..first+second during this callback.
                            // Length was budget-checked; no slice survives the acknowledged write.
                            let chunk = unsafe {
                                std::slice::from_raw_parts((first as *const u8).add(offset), length)
                            };
                            if let Err(error) = output.write_file(token, chunk) {
                                state.error = Some(error);
                                return -1;
                            }
                            offset += length;
                        }
                    }
                }
                1
            }
            native::UCM_NEEDPASSWORDW => {
                let Some(password) = &state.password else {
                    state.error = Some(ArchiveError::PasswordRequired);
                    return -1;
                };
                let Ok(capacity) = usize::try_from(second) else {
                    return -1;
                };
                if first == 0 || capacity > 65536 || password.len() + 1 > capacity {
                    state.error = Some(ArchiveError::ResourceLimit);
                    return -1;
                }
                // SAFETY: documented UCM_NEEDPASSWORDW buffer has second UTF16 slots;
                // checked capacity bounds the copy and terminating zero; no reference escapes.
                unsafe {
                    ptr::copy_nonoverlapping(password.as_ptr(), first as *mut u16, password.len());
                    *((first as *mut u16).add(password.len())) = 0;
                }
                1
            }
            native::UCM_NEEDPASSWORD => {
                state.error = Some(ArchiveError::PasswordRequired);
                -1
            }
            native::UCM_CHANGEVOLUMEW if second == native::RAR_VOL_NOTIFY => {
                let expected = state.volume_names.get(state.next_volume);
                let valid = expected.is_some_and(|expected| {
                    first != 0
                        && expected.len() <= 65536
                        && unsafe {
                            // MAXPATHSIZE callback buffer; compare the complete zero-terminated
                            // path using Windows case semantics before accepting the transition.
                            expected.iter().enumerate().all(|(index, unit)| {
                                let actual = *(first as *const u16).add(index);
                                if *unit < 128 && actual < 128 {
                                    (*unit as u8).eq_ignore_ascii_case(&(actual as u8))
                                } else {
                                    *unit == actual
                                }
                            })
                        }
                });
                if valid {
                    state.next_volume += 1;
                    state.notifying_ansi = true;
                    1
                } else {
                    state.error = Some(ArchiveError::MissingVolume);
                    -1
                }
            }
            native::UCM_CHANGEVOLUME
                if second == native::RAR_VOL_NOTIFY && state.notifying_ansi =>
            {
                state.notifying_ansi = false;
                1
            }
            native::UCM_CHANGEVOLUME | native::UCM_CHANGEVOLUMEW => {
                state.error = Some(ArchiveError::MissingVolume);
                -1
            }
            _ => 0,
        }
    }))
    .unwrap_or(-1)
}
struct Handle(*const native::Handle);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: uniquely owned native handle from RAROpenArchiveEx, closed once.
        unsafe {
            native::RARCloseArchive(self.0);
        }
    }
}
fn native_error(code: i32, encrypted: bool) -> ArchiveError {
    match code {
        native::ERAR_NO_MEMORY => ArchiveError::ResourceLimit,
        native::ERAR_MISSING_PASSWORD => ArchiveError::PasswordRequired,
        native::ERAR_BAD_PASSWORD => ArchiveError::WrongPassword,
        native::ERAR_BAD_DATA if encrypted => ArchiveError::AuthenticationOrDataError,
        native::ERAR_BAD_DATA | native::ERAR_BAD_ARCHIVE => ArchiveError::CorruptArchive,
        native::ERAR_UNKNOWN_FORMAT => ArchiveError::UnsupportedMethod,
        native::ERAR_EOPEN => ArchiveError::MissingVolume,
        _ => ArchiveError::IoFailure,
    }
}
type PendingMetadata = Vec<(uuid::Uuid, crate::metadata::OutputMetadata)>;
type Decoded = (Vec<ArchiveEntry>, u64, u64, PendingMetadata);
pub(super) fn execute(
    request: &WorkerRequest,
    control: &OperationControl,
    output: Option<&mut (dyn CodecOutput + Send)>,
    expected: Option<&[ArchiveEntry]>,
    job_plan: Option<&crate::planning::ExtractionPlan>,
) -> Result<Decoded> {
    let limits = crate::worker_browser::decode_limits(request);
    let decoding = request.operation == "test" || output.is_some();
    let mut path: Vec<u16> = Path::new(&request.path).as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(ArchiveError::UnsafePath);
    }
    path.push(0);
    let context = Box::new(Mutex::new(CallbackState {
        control: control.clone(),
        password: request
            .password
            .as_deref()
            .map(|value| value.encode_utf16().collect()),
        volume_names: request
            .volume_paths
            .iter()
            .skip(1)
            .map(|path| {
                let mut wide: Vec<u16> = Path::new(path).as_os_str().encode_wide().collect();
                wide.push(0);
                wide
            })
            .collect(),
        next_volume: 0,
        notifying_ansi: false,
        limits,
        selected_limit: None,
        entry_bytes: 0,
        total_bytes: 0,
        error: None,
        output,
        token: None,
        pending: Vec::new(),
        decoding_checkpoint: false,
    }));
    // Every listed or extracted transition must consume the exact admitted set.
    let mode = if decoding {
        native::RAR_OM_EXTRACT
    } else {
        native::RAR_OM_LIST
    };
    let mut open = native::OpenArchiveDataEx::new(path.as_ptr(), mode);
    open.callback = Some(callback);
    open.user_data = (&*context as *const Mutex<CallbackState<'_>>) as native::LPARAM;
    // SAFETY: initialized output struct plus live zero-terminated UTF16 path/context.
    let raw = unsafe { native::RAROpenArchiveEx(std::ptr::addr_of_mut!(open)) };
    if raw.is_null() || open.open_result != 0 {
        if !raw.is_null() {
            drop(Handle(raw));
        }
        if let Some(error) = context.lock().map_err(|_| ArchiveError::IoFailure)?.error {
            return Err(error);
        }
        return Err(native_error(
            open.open_result as i32,
            request.password.is_some(),
        ));
    }
    let handle = Handle(raw);
    if open.flags & native::ROADF_VOLUME != 0 && open.flags & native::ROADF_FIRSTVOLUME == 0 {
        return Err(ArchiveError::MissingVolume);
    }
    if let Some(output) = &mut context.lock().map_err(|_| ArchiveError::IoFailure)?.output {
        output.source_checkpoint("codec_opened")?;
    }
    let mut entries = Vec::new();
    let mut retained = 0u64;
    let mut files = 0u64;
    let mut written_bytes = 0u64;
    let mut declared = 0u64;
    loop {
        control.checkpoint()?;
        // SAFETY: all pointer/reserved slots must be null/zero per exact DLL contract.
        let mut header: PackedHeader = unsafe { std::mem::zeroed() };
        // SAFETY: live handle and correctly sized pack1 output; no Rust packed reference escapes.
        let code = unsafe { read_header(handle.0, std::ptr::addr_of_mut!(header)) };
        if code == native::ERAR_END_ARCHIVE {
            break;
        }
        if let Some(error) = context.lock().map_err(|_| ArchiveError::IoFailure)?.error {
            return Err(error);
        }
        if code != 0 {
            return Err(native_error(code, request.password.is_some()));
        }
        if entries.len() as u64 >= limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let filename = header.filename_w; // Copy unaligned packed array before taking a reference.
        let end = filename
            .iter()
            .position(|value| *value == 0)
            .ok_or(ArchiveError::ResourceLimit)?;
        let name =
            String::from_utf16(&filename[..end]).map_err(|_| ArchiveError::CorruptArchive)?;
        retained = retained
            .checked_add(512 + name.len() as u64 * 8)
            .ok_or(ArchiveError::ResourceLimit)?;
        if retained > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let size = (u64::from(header.unp_size_high) << 32) | u64::from(header.unp_size);
        let packed = (u64::from(header.pack_size_high) << 32) | u64::from(header.pack_size);
        let directory = header.flags & native::RHDF_DIRECTORY != 0;
        let encrypted = header.flags & native::RHDF_ENCRYPTED != 0;
        let kind = if header.redir_type != 0 {
            EntryKind::Link
        } else if directory {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        let split = header.flags & (native::RHDF_SPLITBEFORE | native::RHDF_SPLITAFTER) != 0;
        let ordinal = entries.len() as u64;
        let ticks = (u64::from(header.mtime_high) << 32) | u64::from(header.mtime_low);
        let metadata = crate::metadata::OutputMetadata {
            modified: (ticks != 0).then_some(crate::metadata::ModifiedTime::FiletimeUtc { ticks }),
            // The DLL normalizes Windows to public host2; Unix mode is never a Windows mask.
            attributes: (header.host_os == 2).then_some(crate::metadata::WindowsAttributes {
                read_only: header.file_attr & 1 != 0,
                hidden: header.file_attr & 2 != 0,
            }),
        };
        if let Some(expected) = expected {
            let entry = expected
                .get(ordinal as usize)
                .ok_or(ArchiveError::SourceChanged)?;
            if entry.display_path != name
                || entry.kind != kind
                || entry.size != (size != UNKNOWN_SIZE).then_some(size)
                || entry.encrypted != encrypted
                || entry.crc32 != Some(header.file_crc)
                || entry.metadata != metadata
            {
                return Err(ArchiveError::SourceChanged);
            }
        }
        let mut item = metadata_entry(
            EntryId {
                session: request.operation_id,
                ordinal,
            },
            name,
            kind,
            size,
            // A split stream or solid dictionary continuation has no independent packed
            // member ratio. Never present a shared/block byte count as per-file evidence.
            (!split
                && open.flags & native::ROADF_SOLID == 0
                && header.flags & native::RHDF_SOLID == 0)
                .then_some(packed),
            encrypted,
            Some(header.file_crc),
        );
        item.size = (size != UNKNOWN_SIZE).then_some(size);
        item.metadata = metadata;
        item.method = u16::try_from(header.method).map_err(|_| ArchiveError::UnsupportedMethod)?;
        entries.push(item);
        if decoding {
            declared = declared
                .checked_add(if size == UNKNOWN_SIZE { 0 } else { size })
                .ok_or(ArchiveError::ResourceLimit)?;
            if size != UNKNOWN_SIZE && size > limits.max_entry_bytes
                || declared > limits.max_output_bytes
                || u64::from(header.dict_size).saturating_mul(1024) > limits.max_working_bytes
            {
                return Err(ArchiveError::ResourceLimit);
            }
            let mut state = context.lock().map_err(|_| ArchiveError::IoFailure)?;
            state.entry_bytes = 0;
            state.token = None;
            if let Some(output) = state.output.as_mut().filter(|_| {
                request
                    .browser_entry
                    .as_ref()
                    .is_none_or(|selection| selection.ordinal == ordinal)
                    && super::codec_member_wanted(job_plan, entries.last().expect("current header"))
            }) {
                let relative = entries
                    .last()
                    .expect("current header")
                    .display_path
                    .replace('\\', "/");
                let relative = relative.trim_end_matches('/');
                match kind {
                    EntryKind::Directory => output.ensure_directory(relative)?,
                    EntryKind::File => {
                        if let Some(token) = output.begin_extracted_file(
                            relative,
                            (size != UNKNOWN_SIZE).then_some(size),
                        )? {
                            state.pending.push((token, metadata));
                            state.token = Some(token);
                        }
                    }
                    _ => {}
                }
            }
        }
        // SAFETY: RAR_TEST/RAR_SKIP never creates files; null destination/name retained.
        let code = unsafe {
            native::RARProcessFileW(
                handle.0,
                if decoding {
                    native::RAR_TEST
                } else {
                    native::RAR_SKIP
                },
                ptr::null(),
                ptr::null(),
            )
        };
        let state = context.lock().map_err(|_| ArchiveError::IoFailure)?;
        if let Some(error) = state.error {
            return Err(error);
        }
        if code != 0 {
            return Err(native_error(code, encrypted));
        }
        if decoding && kind == EntryKind::File {
            if size != UNKNOWN_SIZE && state.entry_bytes != size {
                return Err(ArchiveError::CorruptArchive);
            }
            if state.output.is_none() || state.token.is_some() {
                files += 1;
                written_bytes += state.entry_bytes;
            }
        }
    }

    if expected.is_some_and(|expected| expected.len() != entries.len()) {
        return Err(ArchiveError::SourceChanged);
    }
    // Native handle must close (including any final callback) before pending ownership is moved.
    drop(handle);
    let mut state = context.lock().map_err(|_| ArchiveError::IoFailure)?;
    if let Some(error) = state.error {
        return Err(error);
    }
    if state.next_volume != state.volume_names.len() {
        return Err(ArchiveError::MissingVolume);
    }
    let bytes = if state.output.is_some() {
        written_bytes
    } else {
        state.total_bytes
    }; // All decoded bytes remain charged.
    let pending = std::mem::take(&mut state.pending);
    Ok((entries, files, bytes, pending))
}

/// Keep one ordered TEST cursor for the drag; a backward ordinal starts one counted replay.
pub(super) fn retained<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    drag: &crate::worker_drag::DragPort,
    set: &volumes::SourceSet,
    entries: std::sync::Arc<[ArchiveEntry]>,
) -> Result<(u64, u64)> {
    let mut handle = Some(open_retained(request, control)?);
    let mut cursor = 0usize;
    let mut volume_cursor = 0usize;
    set.verify(control)?;
    crate::worker_drag::run_codec(
        request,
        control,
        output,
        replies,
        drag,
        |current, demand, native| {
            let selected = crate::worker_browser::selected(current, &entries)?
                .ok_or(ArchiveError::SourceChanged)?;
            let index = usize::try_from(demand.ordinal).map_err(|_| ArchiveError::SourceChanged)?;
            if selected.id().ordinal != demand.ordinal || index >= entries.len() {
                return Err(ArchiveError::SourceChanged);
            }
            let replay = index < cursor;
            if replay {
                drop(handle.take());
                handle = Some(open_retained(request, control)?);
                cursor = 0;
                volume_cursor = 0;
            }
            native.source_checkpoint("codec_opened")?;
            native.source_checkpoint("source_decoding")?;
            let (token, written) = process_retained(
                handle.as_ref().ok_or(ArchiveError::IoFailure)?,
                &mut cursor,
                index,
                request,
                current,
                &entries,
                control,
                native,
                replay,
                &mut volume_cursor,
            )?;
            let result = (|| {
                set.verify(control)?;
                native.source_checkpoint("source_precommit")?;
                selected.metadata.validate()?;
                native.set_file_metadata(token, selected.metadata)?;
                native.commit_file(token)?;
                Ok(written)
            })();
            if result.is_err() {
                native.abort_file(token);
            }
            result
        },
    )
}

struct RetainedHandle {
    native: Handle,
    _path: Vec<u16>,
}

struct InstalledCallback(*const native::Handle);
impl Drop for InstalledCallback {
    fn drop(&mut self) {
        // SAFETY: the native handle remains live until after this callback is detached.
        unsafe {
            native::RARSetCallback(self.0, None, 0);
        }
    }
}
fn open_retained(request: &WorkerRequest, control: &OperationControl) -> Result<RetainedHandle> {
    let mut path: Vec<u16> = Path::new(&request.path).as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(ArchiveError::UnsafePath);
    }
    path.push(0);
    let initial = Box::new(Mutex::new(retained_state(request, control, None, None)));
    let mut open = native::OpenArchiveDataEx::new(path.as_ptr(), native::RAR_OM_EXTRACT);
    open.callback = Some(callback);
    open.user_data = (&*initial as *const Mutex<CallbackState<'_>>) as native::LPARAM;
    control.record_work(|work| work.rar_decoder_opens += 1);
    // SAFETY: path and callback state live until the opening call completes.
    let raw = unsafe { native::RAROpenArchiveEx(std::ptr::addr_of_mut!(open)) };
    if raw.is_null() || open.open_result != 0 {
        if !raw.is_null() {
            drop(Handle(raw));
        }
        if let Some(error) = initial.lock().map_err(|_| ArchiveError::IoFailure)?.error {
            return Err(error);
        }
        return Err(native_error(
            open.open_result as i32,
            request.password.is_some(),
        ));
    }
    let handle = Handle(raw);
    // SAFETY: remove the temporary opening callback before its owner is dropped.
    unsafe {
        native::RARSetCallback(raw, None, 0);
    }
    if open.flags & native::ROADF_VOLUME != 0 && open.flags & native::ROADF_FIRSTVOLUME == 0 {
        return Err(ArchiveError::MissingVolume);
    }
    Ok(RetainedHandle {
        native: handle,
        _path: path,
    })
}

fn retained_state<'a>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: Option<&'a mut (dyn CodecOutput + Send)>,
    selected_limit: Option<u64>,
) -> CallbackState<'a> {
    CallbackState {
        control: control.clone(),
        password: request
            .password
            .as_deref()
            .map(|value| value.encode_utf16().collect()),
        limits: crate::worker_browser::decode_limits(request),
        entry_bytes: 0,
        total_bytes: 0,
        error: None,
        selected_limit,
        output,
        token: None,
        pending: Vec::new(),
        decoding_checkpoint: false,
        volume_names: request
            .volume_paths
            .iter()
            .skip(1)
            .map(|path| {
                let mut name: Vec<u16> = Path::new(path).as_os_str().encode_wide().collect();
                name.push(0);
                name
            })
            .collect(),
        next_volume: 0,
        notifying_ansi: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn process_retained<W: Write + Send>(
    handle: &RetainedHandle,
    cursor: &mut usize,
    index: usize,
    original: &WorkerRequest,
    current: &WorkerRequest,
    entries: &[ArchiveEntry],
    control: &OperationControl,
    output: &mut PrivateOutput<'_, W>,
    replay: bool,
    volume_cursor: &mut usize,
) -> Result<(uuid::Uuid, u64)> {
    let mut state = retained_state(
        original,
        control,
        Some(output),
        Some(current.limits.max_entry_bytes),
    );
    state.next_volume = *volume_cursor;
    let context = Box::new(Mutex::new(state));
    // SAFETY: callback state stays boxed and live until InstalledCallback is dropped.
    unsafe {
        native::RARSetCallback(
            handle.native.0,
            Some(callback),
            (&*context as *const Mutex<CallbackState<'_>>) as native::LPARAM,
        );
    }
    let attached = InstalledCallback(handle.native.0);
    let mut result = None;
    while *cursor <= index {
        control.checkpoint()?;
        // SAFETY: zeroed pack1 header and owned live native TEST handle.
        let mut header: PackedHeader = unsafe { std::mem::zeroed() };
        let code = unsafe { read_header(handle.native.0, std::ptr::addr_of_mut!(header)) };
        if let Some(error) = context.lock().map_err(|_| ArchiveError::IoFailure)?.error {
            return Err(error);
        }
        if code != 0 {
            return Err(if code == native::ERAR_END_ARCHIVE {
                ArchiveError::SourceChanged
            } else {
                native_error(code, original.password.is_some())
            });
        }
        let expected = entries.get(*cursor).ok_or(ArchiveError::SourceChanged)?;
        let name = header.filename_w;
        let end = name
            .iter()
            .position(|unit| *unit == 0)
            .ok_or(ArchiveError::ResourceLimit)?;
        let name = String::from_utf16(&name[..end]).map_err(|_| ArchiveError::CorruptArchive)?;
        let size = (u64::from(header.unp_size_high) << 32) | u64::from(header.unp_size);
        let kind = if header.redir_type != 0 {
            EntryKind::Link
        } else if header.flags & native::RHDF_DIRECTORY != 0 {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        let ticks = (u64::from(header.mtime_high) << 32) | u64::from(header.mtime_low);
        let metadata = crate::metadata::OutputMetadata {
            modified: (ticks != 0).then_some(crate::metadata::ModifiedTime::FiletimeUtc { ticks }),
            attributes: (header.host_os == 2).then_some(crate::metadata::WindowsAttributes {
                read_only: header.file_attr & 1 != 0,
                hidden: header.file_attr & 2 != 0,
            }),
        };
        if expected.id().ordinal != *cursor as u64
            || expected.display_path != name
            || expected.kind != kind
            || expected.size != (size != UNKNOWN_SIZE).then_some(size)
            || expected.encrypted != (header.flags & native::RHDF_ENCRYPTED != 0)
            || expected.crc32 != Some(header.file_crc)
            || expected.metadata != metadata
            || u16::try_from(header.method).ok() != Some(expected.method)
            || expected.compressed_size.is_some_and(|packed| {
                packed != ((u64::from(header.pack_size_high) << 32) | u64::from(header.pack_size))
            })
        {
            return Err(ArchiveError::SourceChanged);
        }
        if u64::from(header.dict_size).saturating_mul(1024) > original.limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let selected = *cursor == index;
        let mut state = context.lock().map_err(|_| ArchiveError::IoFailure)?;
        state.entry_bytes = 0;
        state.token = None;
        if selected {
            if expected
                .size
                .is_some_and(|value| value > current.limits.max_entry_bytes)
            {
                return Err(ArchiveError::ResourceLimit);
            }
            let relative = expected.display_path.replace('\\', "/");
            let token = state
                .output
                .as_mut()
                .ok_or(ArchiveError::IoFailure)?
                .begin_extracted_file(relative.trim_end_matches('/'), expected.size)?
                .ok_or(ArchiveError::OutputCollision)?;
            state.pending.push((token, metadata));
            state.token = Some(token);
        }
        drop(state);
        // SAFETY: RAR_TEST advances solid history without a filesystem destination.
        let code = unsafe {
            native::RARProcessFileW(handle.native.0, native::RAR_TEST, ptr::null(), ptr::null())
        };
        let mut state = context.lock().map_err(|_| ArchiveError::IoFailure)?;
        if let Some(error) = state.error {
            return Err(error);
        }
        if code != 0 {
            return Err(native_error(code, expected.encrypted));
        }
        if size != UNKNOWN_SIZE && state.entry_bytes != size {
            return Err(ArchiveError::CorruptArchive);
        }
        if selected {
            result = Some((
                state.token.ok_or(ArchiveError::IoFailure)?,
                state.entry_bytes,
            ));
            state.pending.clear();
        } else if replay {
            control.record_work(|work| work.rar_replayed_entries += 1);
        }
        *cursor += 1;
    }
    *volume_cursor = context
        .lock()
        .map_err(|_| ArchiveError::IoFailure)?
        .next_volume;
    drop(attached);
    drop(context);
    result.ok_or(ArchiveError::SourceChanged)
}
