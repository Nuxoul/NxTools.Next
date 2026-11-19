//! True NTFS last-write metadata. Journal event time is deliberately never an input.

#[cfg(any(windows, test))]
pub(super) fn filetime_to_unix_ms(ticks: i64) -> Option<i64> {
    const UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;
    // Zero/negative NTFS times and dates before the Unix epoch are unknown to this view.
    let elapsed = ticks.checked_sub(UNIX_EPOCH_TICKS)?;
    (elapsed > 0)
        .then_some(elapsed / 10_000)
        .filter(|ms| *ms > 0)
}

#[cfg(all(test, windows))]
thread_local! {
    static ATTRIBUTE_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(all(test, windows))]
pub(super) fn attribute_read_count() -> u64 {
    ATTRIBUTE_READS.with(std::cell::Cell::get)
}

#[cfg(windows)]
pub(crate) use native::read_last_write_by_id;

#[cfg(windows)]
#[allow(unsafe_code)]
mod native {
    use std::{ffi::c_void, mem::size_of, ptr::null};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FileBasicInfo, FileIdType, GetFileInformationByHandleEx,
        OpenFileById,
    };

    pub(crate) fn read_last_write_by_id(volume: HANDLE, reference: u64) -> Option<i64> {
        #[cfg(test)]
        super::ATTRIBUTE_READS.with(|count| count.set(count.get() + 1));
        let descriptor = FILE_ID_DESCRIPTOR {
            dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: FileIdType,
            // Retain sequence bits: a reused MFT slot must not supply another file's time.
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                FileId: reference as i64,
            },
        };
        // SAFETY: descriptor is initialized, volume remains owned by the caller. Attribute-only
        // access and sharing do not open file contents. Do not follow a reparse target.
        let file = unsafe {
            OpenFileById(
                volume,
                &descriptor,
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            )
        };
        if file == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut info = FILE_BASIC_INFO::default();
        // SAFETY: output points to a correctly sized live structure; close exactly this handle.
        let success = unsafe {
            let success = GetFileInformationByHandleEx(
                file,
                FileBasicInfo,
                (&raw mut info).cast::<c_void>(),
                size_of::<FILE_BASIC_INFO>() as u32,
            );
            let _ = CloseHandle(file);
            success
        };
        if success == 0 {
            None
        } else {
            super::filetime_to_unix_ms(info.LastWriteTime)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn w4_filetime_conversion_preserves_true_last_write_and_rejects_unknown() {
        const EPOCH: i64 = 116_444_736_000_000_000;
        assert_eq!(filetime_to_unix_ms(EPOCH + 12_345 * 10_000), Some(12_345));
        assert_eq!(filetime_to_unix_ms(EPOCH + 1), None);
        assert_eq!(filetime_to_unix_ms(EPOCH), None);
        assert_eq!(filetime_to_unix_ms(0), None);
        assert_eq!(filetime_to_unix_ms(i64::MIN), None);
        // A deliberately restored older last-write stays older (no monotonic max with event time).
        assert!(filetime_to_unix_ms(EPOCH + 100_000) < filetime_to_unix_ms(EPOCH + 200_000));
    }
}
