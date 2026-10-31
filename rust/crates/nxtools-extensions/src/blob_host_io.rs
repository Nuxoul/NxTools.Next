//! Host-only byte seam; no paths, handles, protocol reader or independent cleanup owner.
use super::*;

/// Unpublished response reservation. Dropping it retires partial bytes while actual IO/backing
/// references keep the original global AND invocation permits until physical cleanup.
pub struct HostBlobWriter {
    run: BlobRun,
    backing: Arc<Backing>,
    published: bool,
}
impl Drop for HostBlobWriter {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        let removed = {
            let mut state = self.run.0.state.lock().unwrap_or_else(|p| p.into_inner());
            let removed = state.blobs.remove(&self.backing.id);
            if let Some(backing) = &removed {
                state.bytes -= backing.length;
            }
            removed
        };
        drop(removed);
        // The same Backing::drop/Root owner performs and accounts actual cleanup.
    }
}
fn write_host_chunk(backing: Arc<Backing>, bytes: Vec<u8>) -> Result<(), BlobError> {
    let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
    if state.sealed || !backing.host_owned {
        return Err(BlobError::InvalidTransfer);
    }
    let end = state
        .offset
        .checked_add(bytes.len())
        .ok_or(BlobError::ResourceLimit)?;
    if end > backing.length {
        return Err(BlobError::ResourceLimit);
    }
    state
        .file
        .as_mut()
        .ok_or(BlobError::Io)?
        .write_all(&bytes)
        .map_err(|_| BlobError::Io)?;
    state.digest.update(&bytes);
    state.offset = end;
    Ok(())
}
impl HostBlobWriter {
    /// One decoded host chunk, never larger than the original data-plane chunk budget.
    pub async fn write(&mut self, bytes: Vec<u8>) -> Result<(), BlobError> {
        self.run.current()?;
        if bytes.is_empty() || bytes.len() > MAX_CHUNK_BYTES {
            return Err(BlobError::InvalidParams);
        }
        let backing = self.run.backing(self.backing.id)?;
        self.run.io(move || write_host_chunk(backing, bytes)).await
    }
    /// Publish only an entirely completed response. Reservation remains its maximum capacity until
    /// backing deletion, but the immutable descriptor/original ACK reader expose actual bytes only.
    pub async fn seal(mut self) -> Result<BlobDescriptor, BlobError> {
        let backing = self.run.backing(self.backing.id)?;
        let descriptor = self
            .run
            .io(move || {
                let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
                if state.sealed || !backing.host_owned {
                    return Err(BlobError::InvalidTransfer);
                }
                state
                    .file
                    .as_mut()
                    .ok_or(BlobError::Io)?
                    .flush()
                    .map_err(|_| BlobError::Io)?;
                let blob = BlobDescriptor {
                    blob_id: backing.id,
                    length: state.offset,
                    media_type: backing.media_type.clone(),
                    sha256: format!("{:x}", state.digest.clone().finalize()),
                };
                backing
                    .completed
                    .set(blob.clone())
                    .map_err(|_| BlobError::InvalidTransfer)?;
                state.sealed = true;
                backing.sealed.store(true, Ordering::Release);
                Ok(blob)
            })
            .await?;
        self.published = true;
        Ok(descriptor)
    }
}
impl BlobRun {
    /// Reserve finite response capacity before network contact. A cancellation during file creation
    /// drops the unpublished guard; a surviving blocking IO closure retains all original permits.
    pub async fn host_writer(
        &self,
        capacity: usize,
        media_type: String,
    ) -> Result<HostBlobWriter, BlobError> {
        let backing = self.reserve_backing(capacity, media_type, "0".repeat(64), true)?;
        let writer = HostBlobWriter {
            run: self.clone(),
            backing: backing.clone(),
            published: false,
        };
        self.open_backing(backing).await?;
        tracing::info!(event="extension.blob.accepted",correlation_id=%self.0.identity.correlation_id,run_id=%self.0.identity.run_id,byte_count=capacity,"temporary host blob admitted");
        Ok(writer)
    }
    /// Bounded same-invocation sealed body for a host consumer. It creates no wire transfer, exposes
    /// no file handle, and cannot read a foreign/unsealed/retired blob or bypass the IO owner.
    pub async fn host_read_sealed(&self, id: Uuid, maximum: usize) -> Result<Vec<u8>, BlobError> {
        let backing = self.backing(id)?;
        if !backing.sealed.load(Ordering::Acquire) {
            return Err(BlobError::InvalidTransfer);
        }
        let length = backing.length();
        if maximum > MAX_BLOB_BYTES || length > maximum {
            return Err(BlobError::ResourceLimit);
        }
        self.io(move || {
            let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
            let file = state.file.as_mut().ok_or(BlobError::Io)?;
            file.seek(SeekFrom::Start(0)).map_err(|_| BlobError::Io)?;
            let mut bytes = vec![0; length];
            file.read_exact(&mut bytes).map_err(|_| BlobError::Io)?;
            Ok(bytes)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> crate::InvocationIdentity {
        crate::InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.host-io".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        }
    }
    #[tokio::test]
    async fn expansion_http_host_blob_cancelled_write_keeps_actual_io_and_capacity_until_unlock() {
        let root = tempfile::tempdir().expect("owned held IO");
        let store = BlobStore::new(root.path().join("backing"));
        let run = store.invocation(identity(), Uuid::new_v4());
        let writer = run
            .host_writer(1024, "text/plain".into())
            .await
            .expect("response reservation");
        let backing = writer.backing.clone();
        let (entered, arrival) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let retained = run.clone();
        let task = tokio::spawn(async move {
            let _writer = writer;
            retained
                .io(move || {
                    entered.send(()).expect("actual host IO entered");
                    wait.recv_timeout(Duration::from_secs(3))
                        .expect("bounded independent IO unlock");
                    write_host_chunk(backing, b"abc".to_vec())
                })
                .await
        });
        arrival.await.expect("actual blocking host write started");
        run.retire();
        task.abort();
        let _ = task.await;
        assert!(
            !run.quiescent(),
            "logical cancellation is not actual IO completion"
        );
        assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES - 1024);
        release.send(()).expect("finish actual IO");
        tokio::time::timeout(Duration::from_secs(3), async {
            while store.available_bytes() != MAX_GLOBAL_BLOB_BYTES {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("same cleanup owner releases real capacity");
        assert!(run.quiescent());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn expansion_http_host_blob_failed_delete_retains_reserved_capacity_not_actual_length() {
        use std::os::windows::fs::OpenOptionsExt as _;
        let root = tempfile::tempdir().expect("owned denied delete");
        let directory = root.path().join("backing");
        let store = BlobStore::new(directory.clone());
        let run = store.invocation(identity(), Uuid::new_v4());
        let mut writer = run
            .host_writer(1024, "text/plain".into())
            .await
            .expect("capacity");
        writer
            .write(b"abc".to_vec())
            .await
            .expect("partial response");
        let held = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(directory.join(writer.backing.id.to_string()))
            .expect("deny actual deletion");
        drop(writer);
        assert_eq!(store.cleanup().await, Err(BlobError::Io));
        assert_eq!(
            store.available_bytes(),
            MAX_GLOBAL_BLOB_BYTES - 1024,
            "charge full reservation until deletion"
        );
        assert!(
            run.quiescent(),
            "physical cleanup is distinct from script data quiescence"
        );
        drop(held);
        store.cleanup().await.expect("original physical cleanup");
        assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    }
}
