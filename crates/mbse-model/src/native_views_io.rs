//! Independently acknowledged view writes. Native Save always happens first.
//! No cross-file atomicity, noncooperating-writer CAS or power-loss guarantee.
use crate::{
    local_input,
    native_views::{Capture, Document, NativeBinding, Views, MAX_BYTES},
    stage1::Freshness,
    stage1_io::{NativeCheckpoint, Session},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Bound,
    Recovery,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ticket {
    session: String,
    instance: String,
    request: u64,
    revision: u64,
    document: String,
    native_freshness: Freshness,
    native_request: Option<u64>,
    kind: Kind,
    path: PathBuf,
    hash: String,
    binding: Option<NativeBinding>,
}
/// Private frozen bytes/ticket; callers cannot forge a receipt or change a job.
pub struct Job {
    ticket: Ticket,
    bytes: Vec<u8>,
    expected: Option<String>,
}
/// A non-deserializable capability to abandon only this unstarted job.
pub struct StartToken {
    ticket: Ticket,
}
impl Job {
    pub fn start_token(&self) -> StartToken {
        StartToken {
            ticket: self.ticket.clone(),
        }
    }
}
pub struct Receipt {
    ticket: Ticket,
    outcome: Result<(), String>,
}
impl Receipt {
    pub fn error(&self) -> Option<&str> {
        self.outcome.as_ref().err().map(String::as_str)
    }
}
#[derive(Clone, Debug)]
pub struct RetainedInput {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub reason: String,
}
struct Acknowledged {
    path: PathBuf,
    disk_hash: String,
    content_digest: String,
    revision: u64,
    native_freshness: Freshness,
    binding: Option<NativeBinding>,
}
#[derive(Default)]
pub struct Io {
    next: u64,
    pending: Option<Ticket>,
    saved: Option<Acknowledged>,
    recovered: Option<Acknowledged>,
    protected: BTreeSet<PathBuf>,
    pub retained: Vec<RetainedInput>,
    pub last_error: Option<String>,
}
impl Io {
    pub fn failed_start(&mut self, session: &Session, token: StartToken, error: String) -> bool {
        if self.pending.as_ref() != Some(&token.ticket)
            || token.ticket.session != session.session_id().0
            || token.ticket.instance != session.instance_id().0
        {
            return false;
        }
        self.pending = None;
        self.last_error = Some(format!("VIEWS_WORKER_START: {error}"));
        true
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    /// Cheap in-memory status hint. This does not prove current disk contents.
    pub fn is_saved(&self, session: &Session, views: &Views) -> bool {
        self.saved.as_ref().is_some_and(|s| {
            s.revision == views.document().revision()
                && s.native_freshness == session.editor.freshness()
                && s.binding.as_ref().is_some_and(|b| {
                    session.accepted_native_artifact_sha256() == Some(b.artifact_sha256.as_str())
                })
        })
    }
    /// Cheap recovery status hint; verification is separate before navigation.
    pub fn recovery_current(&self, session: &Session, views: &Views) -> bool {
        self.recovered.as_ref().is_some_and(|s| {
            s.revision == views.document().revision()
                && s.native_freshness == session.editor.freshness()
        })
    }
    pub fn saved_path(&self) -> Option<&Path> {
        self.saved.as_ref().map(|s| s.path.as_path())
    }
    /// Only this checked path may authorize leaving the live layout. It checks
    /// the exact acknowledged disk bytes and the canonical current content.
    pub fn verify_preserved(
        &self,
        session: &Session,
        views: &Views,
        kind: Kind,
    ) -> Result<(), String> {
        if self.pending() {
            return Err("VIEWS_WRITE_PENDING".into());
        }
        let ack = match kind {
            Kind::Bound => self.saved.as_ref(),
            Kind::Recovery => self.recovered.as_ref(),
        }
        .ok_or("VIEWS_NOT_ACKNOWLEDGED")?;
        if ack.revision != views.document().revision()
            || ack.native_freshness != session.editor.freshness()
        {
            return Err("VIEWS_NEWER_STATE_UNSAVED".into());
        }
        let bytes = local_input::read(&ack.path, MAX_BYTES, "VIEWS_CONTINUE")?;
        if hash(&bytes) != ack.disk_hash {
            return Err("VIEWS_CONTINUE_DISK_CHANGED".into());
        }
        let current = match kind {
            Kind::Bound => {
                let checkpoint = accepted_native(session)?;
                if !ack.binding.as_ref().is_some_and(|b| b.matches(&checkpoint)) {
                    return Err("VIEWS_CONTINUE_NATIVE_CHANGED".into());
                }
                views.freeze(&session.editor, &checkpoint)?
            }
            Kind::Recovery => views.recovery(&session.editor)?,
        };
        if current.digest()? != ack.content_digest {
            return Err("VIEWS_CONTINUE_CONTENT_CHANGED".into());
        }
        Ok(())
    }
    pub fn begin(
        &mut self,
        session: &Session,
        views: &Views,
        path: &Path,
        kind: Kind,
    ) -> Result<Job, String> {
        if self.pending() {
            return Err("VIEWS_WRITE_PENDING".into());
        }
        let checkpoint = if kind == Kind::Bound {
            let checkpoint = session
                .native_checkpoint()?
                .ok_or("VIEWS_NATIVE_SAVE_REQUIRED")?;
            if !checkpoint.is_current(session) {
                return Err("VIEWS_NATIVE_CHECKPOINT_HISTORICAL".into());
            }
            Some(checkpoint)
        } else {
            None
        };
        let document = if let Some(checkpoint) = &checkpoint {
            views.freeze(&session.editor, checkpoint)?
        } else {
            views.recovery(&session.editor)?
        };
        self.prepare(session, document, path, kind)
    }
    pub fn begin_captured(
        &mut self,
        session: &Session,
        capture: Capture,
        path: &Path,
        kind: Kind,
    ) -> Result<Job, String> {
        if self.pending() {
            return Err("VIEWS_WRITE_PENDING".into());
        }
        let checkpoint = if kind == Kind::Bound {
            Some(accepted_native(session)?)
        } else {
            None
        };
        let document = capture.document(&session.editor, checkpoint.as_ref())?;
        self.prepare(session, document, path, kind)
    }
    fn prepare(
        &mut self,
        session: &Session,
        document: Document,
        path: &Path,
        kind: Kind,
    ) -> Result<Job, String> {
        let path = destination(path)?;
        if self.protected.contains(&path) {
            return Err("VIEWS_PROTECTED_INPUT: choose a fresh destination".into());
        }
        if session.path.as_ref() == Some(&path) {
            return Err("VIEWS_NATIVE_PATH_FORBIDDEN".into());
        }
        let bytes = document.encoded()?;
        let expected = if kind == Kind::Bound {
            self.saved
                .as_ref()
                .filter(|s| s.path == path)
                .map(|s| s.disk_hash.clone())
        } else {
            None
        };
        check_target(&path, expected.as_deref())?;
        let request = self.next.checked_add(1).ok_or("VIEWS_REQUEST_EXHAUSTED")?;
        let ticket = Ticket {
            session: session.session_id().0.clone(),
            instance: session.instance_id().0.clone(),
            request,
            revision: document.revision(),
            document: document.document_id.0.clone(),
            native_freshness: session.editor.freshness(),
            native_request: if kind == Kind::Bound {
                session
                    .native_checkpoint()?
                    .and_then(|c| c.accepted_request().map(|r| r.0))
            } else {
                None
            },
            kind,
            path,
            hash: hash(&bytes),
            binding: document.binding.clone(),
        };
        self.next = request;
        self.pending = Some(ticket.clone());
        self.last_error = None;
        Ok(Job {
            ticket,
            bytes,
            expected,
        })
    }
    /// true means consumed, including failures. Only is_saved/recovery_current
    /// can authorize continuation; stale completions cannot clear a newer job.
    pub fn complete(&mut self, session: &Session, views: &Views, result: Receipt) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        if *pending != result.ticket
            || pending.session != session.session_id().0
            || pending.instance != session.instance_id().0
            || pending.document != session.editor.snapshot.document_id.0
        {
            return false;
        }
        let ticket = self.pending.take().expect("matching pending");
        let outcome = result.outcome.and_then(|()| {
            let bytes = local_input::read(&ticket.path, MAX_BYTES, "VIEWS_ACK")?;
            if hash(&bytes) != ticket.hash {
                return Err("VIEWS_ACK_DISK_CHANGED".into());
            }
            match ticket.kind {
                Kind::Bound => {
                    let checkpoint = session
                        .native_checkpoint()?
                        .ok_or("VIEWS_ACK_NATIVE_MISSING")?;
                    if checkpoint.accepted_request().map(|r| r.0) != ticket.native_request {
                        return Err("VIEWS_ACK_NATIVE_REQUEST_CHANGED".into());
                    }
                    if !ticket
                        .binding
                        .as_ref()
                        .is_some_and(|b| b.matches(&checkpoint))
                    {
                        return Err("VIEWS_ACK_NATIVE_CHANGED".into());
                    }
                    // The frozen revision may be older. Its digest is recorded,
                    // but is_saved compares to the live presentation independently.
                    self.saved = Some(Acknowledged {
                        path: ticket.path.clone(),
                        disk_hash: ticket.hash.clone(),
                        content_digest: ticket.hash.clone(),
                        revision: ticket.revision,
                        native_freshness: ticket.native_freshness.clone(),
                        binding: ticket.binding.clone(),
                    });
                }
                Kind::Recovery => {
                    self.recovered = Some(Acknowledged {
                        path: ticket.path.clone(),
                        disk_hash: ticket.hash.clone(),
                        content_digest: ticket.hash.clone(),
                        revision: ticket.revision,
                        native_freshness: ticket.native_freshness.clone(),
                        binding: None,
                    });
                }
            }
            Ok(())
        });
        self.last_error = outcome.err();
        let _ = views; // live revision is deliberately not acknowledged here
        true
    }
    /// Every local layout replacement crosses this preservation boundary.
    /// Cached status and deserialized revisions never authorize replacement.
    fn verify_replacement(&self, session: &Session, views: &Views) -> Result<(), String> {
        if self.pending() || session.pending() {
            return Err(
                "VIEWS_WRITE_PENDING: wait or cancel navigation; live layout retained".into(),
            );
        }
        if views.is_pristine_default(&session.editor) {
            return Ok(());
        }
        if self.verify_preserved(session, views, Kind::Bound).is_ok()
            || self
                .verify_preserved(session, views, Kind::Recovery)
                .is_ok()
        {
            return Ok(());
        }
        Err("VIEWS_REPLACEMENT_REQUIRES_PRESERVATION: save or keep recovery to a fresh destination, verify it, then retry Open; cancel keeps current layout and history".into())
    }
    /// Explicit path only; failed/future/mismatched bytes remain available and
    /// protected. No directory scan, repair or automatic rewrite takes place.
    pub fn open(
        &mut self,
        session: &Session,
        views: &mut Views,
        path: &Path,
        recovery: bool,
    ) -> Result<(), String> {
        self.verify_replacement(session, views)?;
        let path = destination(path)?;
        let bytes = local_input::read(&path, MAX_BYTES, "VIEWS_OPEN")?;
        let result = if recovery {
            Views::restore(&bytes, &session.editor)
        } else {
            session
                .native_checkpoint()
                .and_then(|c| c.ok_or_else(|| "VIEWS_NATIVE_SAVE_REQUIRED".into()))
                .and_then(|c| Views::decode(&bytes, &session.editor, &c))
        };
        match result {
            Ok(candidate) => {
                let digest = hash(&bytes);
                // Noncanonical but valid input is tracked by its canonical
                // content digest separately from the exact prior disk hash.
                let ack = Acknowledged {
                    path: path.clone(),
                    disk_hash: digest,
                    content_digest: if recovery {
                        candidate.recovery(&session.editor)?.digest()?
                    } else {
                        candidate.document().digest()?
                    },
                    revision: candidate.document().revision(),
                    native_freshness: session.editor.freshness(),
                    binding: candidate.document().binding.clone(),
                };
                // A replacement starts a new acknowledgment ledger. Opposite
                // kinds cannot certify unrelated layouts at a colliding revision.
                self.saved = None;
                self.recovered = None;
                if recovery {
                    self.recovered = Some(ack);
                } else {
                    self.saved = Some(ack);
                }
                *views = candidate;
                self.last_error = None;
                Ok(())
            }
            Err(reason) => {
                self.protected.insert(path.clone());
                self.retained.push(RetainedInput {
                    path,
                    bytes,
                    reason: reason.clone(),
                });
                // At most one full bounded failed input is retained in memory;
                // every original path stays protected and unchanged on disk.
                if self.retained.len() > 1 {
                    self.retained.remove(0);
                }
                self.last_error = Some(reason.clone());
                Err(reason)
            }
        }
    }
}
fn destination(path: &Path) -> Result<PathBuf, String> {
    local_input::validate_path(path, "VIEWS")?;
    if !path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|s| s.ends_with(".mbse-views.json"))
    {
        return Err("VIEWS_EXTENSION: use .mbse-views.json".into());
    }
    let parent = fs::canonicalize(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(|e| e.to_string())?;
    Ok(parent.join(path.file_name().ok_or("VIEWS_NAME")?))
}
fn check_target(path: &Path, expected: Option<&str>) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err("VIEWS_UNSAFE_TARGET".into());
            }
            let expected = expected.ok_or("VIEWS_EXISTING_UNOWNED_TARGET")?;
            if hash(&local_input::read(path, MAX_BYTES, "VIEWS_TARGET")?) != expected {
                return Err("VIEWS_DISK_CONFLICT".into());
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && expected.is_none() => Ok(()),
        Err(e) => Err(format!("VIEWS_TARGET: {e}")),
    }
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub fn execute(job: Job) -> Receipt {
    let outcome = (|| {
        let path = &job.ticket.path;
        let parent = path.parent().ok_or("VIEWS_PARENT")?;
        let lock_path = path.with_extension("json.lock");
        let mut lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| format!("VIEWS_BUSY: {e}"))?;
        let _guard = Lock(lock_path);
        lock.write_all(job.ticket.hash.as_bytes())
            .map_err(|e| e.to_string())?;
        lock.sync_all().map_err(|e| e.to_string())?;
        check_target(path, job.expected.as_deref())?;
        let options = craft_atomic::WriteOptions::default()
            .with_new_file_permissions(craft_atomic::NewFilePermissions::Private);
        if job.expected.is_some() {
            // Retain byte-exact prior presentation before replacing an owned
            // file. This private staging directory remains as local history.
            let prior = local_input::read(path, MAX_BYTES, "VIEWS_PRIOR")?;
            let backup = tempfile::Builder::new()
                .prefix(".mbse-views-prior-")
                .tempdir_in(parent)
                .map_err(|e| e.to_string())?;
            options
                .atomic_write(&backup.path().join("prior.mbse-views.json"), &prior)
                .map_err(|e| e.to_string())?;
            let _retained = backup.keep();
            check_target(path, job.expected.as_deref())?;
            options
                .atomic_write(path, &job.bytes)
                .map_err(|e| e.to_string())?;
        } else {
            // Private Craft staging, followed by an exclusive leaf hard-link.
            // No fallback that could replace a racing arrival or symlink.
            let stage = tempfile::Builder::new()
                .prefix(".mbse-views-stage-")
                .tempdir_in(parent)
                .map_err(|e| e.to_string())?;
            let staged = stage.path().join("payload");
            options
                .atomic_write(&staged, &job.bytes)
                .map_err(|e| e.to_string())?;
            fs::hard_link(&staged, path).map_err(|e| format!("VIEWS_NEW_LEAF: {e}"))?;
        }
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        if hash(&local_input::read(path, MAX_BYTES, "VIEWS_VERIFY")?) != job.ticket.hash {
            return Err("VIEWS_POST_WRITE_CONFLICT".into());
        }
        Ok(())
    })();
    Receipt {
        ticket: job.ticket,
        outcome,
    }
}

/// Matching an accepted checkpoint is intentionally separate from the public
/// native IoTicket. A failed native completion never yields a views job.
pub fn accepted_native(session: &Session) -> Result<NativeCheckpoint, String> {
    let checkpoint = session
        .native_checkpoint()?
        .ok_or("VIEWS_NATIVE_SAVE_REQUIRED")?;
    if !checkpoint.is_current(session) {
        return Err("VIEWS_NATIVE_CHECKPOINT_HISTORICAL".into());
    }
    Ok(checkpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_views::{Geometry, Layout, NativeAction},
        stage1::{Command, RawKey, RecordId},
        stage1_io,
    };
    fn area() -> tempfile::TempDir {
        let root =
            std::env::var_os("TMPDIR").expect("Views tests require explicit workspace TMPDIR");
        assert!(Path::new(&root).starts_with("/workspace") || cfg!(windows));
        tempfile::Builder::new()
            .prefix("native-views-test-")
            .tempdir_in(root)
            .unwrap()
    }
    fn native_save(session: &mut Session, path: &Path) {
        let job = session.begin_save(path).unwrap();
        let result = stage1_io::execute_write(job);
        assert!(result.succeeded(), "{:?}", result.error());
        assert!(session.complete(result));
    }
    fn move_layout(session: &Session, views: &mut Views) {
        let id = *views.document().occurrences.keys().next().unwrap();
        let stamp = views.stamp(&session.editor);
        views
            .apply(
                &session.editor,
                &stamp,
                Layout::Move(id, Geometry::at(123., 456.)),
            )
            .unwrap();
    }
    #[test]
    fn bound_save_requires_actual_native_ack_and_round_trips_two_views() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        let mut views = Views::new(&session.editor).unwrap();
        let stamp = views.stamp(&session.editor);
        let outcome = views
            .dispatch(
                &mut session.editor,
                &stamp,
                NativeAction::Command(Command::Add {
                    kind: crate::stage1::RecordKind::Mission,
                    owner: RecordId(1),
                    name: "Shared mission".into(),
                }),
            )
            .unwrap();
        let mission = match outcome {
            crate::native_views::NativeOutcome::Edit(o) => o.created.unwrap(),
            _ => panic!("creation outcome"),
        };
        let view_ids: Vec<_> = views.document().views.keys().copied().collect();
        for view in view_ids {
            let stamp = views.stamp(&session.editor);
            views
                .apply(
                    &session.editor,
                    &stamp,
                    Layout::Reuse {
                        view,
                        record: mission,
                        geometry: Geometry::at(70., 90.),
                    },
                )
                .unwrap();
        }
        let stamp = views.stamp(&session.editor);
        views
            .dispatch(
                &mut session.editor,
                &stamp,
                NativeAction::Command(Command::EditText {
                    key: RawKey {
                        record: mission,
                        field: "name".into(),
                        item: None,
                    },
                    text: "Renamed shared mission".into(),
                }),
            )
            .unwrap();
        let mut io = Io::default();
        let path = area.path().join("layout.mbse-views.json");
        assert!(io.begin(&session, &views, &path, Kind::Bound).is_err());
        native_save(&mut session, &area.path().join("model.kernspan"));
        let job = io.begin(&session, &views, &path, Kind::Bound).unwrap();
        assert!(io.complete(&session, &views, execute(job)));
        assert!(io.is_saved(&session, &views));
        io.verify_preserved(&session, &views, Kind::Bound).unwrap();
        let reopened = Session::open(area.path(), session.path.as_ref().unwrap()).unwrap();
        let mut reopened_views = Views::new(&reopened.editor).unwrap();
        let mut reopened_io = Io::default();
        reopened_io
            .open(&reopened, &mut reopened_views, &path, false)
            .unwrap();
        assert_eq!(reopened_views.document().views.len(), 2);
        assert_eq!(
            reopened_views
                .document()
                .occurrences
                .values()
                .filter(|o| o.record == mission)
                .count(),
            2
        );
        assert_eq!(
            reopened.editor.snapshot.records[&mission].name,
            "Renamed shared mission"
        );
        reopened_io
            .verify_preserved(&reopened, &reopened_views, Kind::Bound)
            .unwrap();
    }
    #[test]
    fn same_snapshot_different_artifact_is_refused_and_original_bytes_retained() {
        let area = area();
        let native = area.path().join("model.kernspan");
        let path = area.path().join("layout.mbse-views.json");
        let mut session = Session::new(area.path()).unwrap();
        native_save(&mut session, &native);
        let views = Views::new(&session.editor).unwrap();
        let mut io = Io::default();
        let job = io.begin(&session, &views, &path, Kind::Bound).unwrap();
        io.complete(&session, &views, execute(job));
        let original = fs::read(&path).unwrap();
        let mut changed = fs::read(&native).unwrap();
        changed.push(b'\n');
        fs::write(&native, changed).unwrap();
        let session = Session::open(area.path(), &native).unwrap();
        let mut default = Views::new(&session.editor).unwrap();
        let before = default.document().digest().unwrap();
        let mut other = Io::default();
        assert!(other.open(&session, &mut default, &path, false).is_err());
        assert_eq!(other.retained[0].bytes, original);
        assert_eq!(default.document().digest().unwrap(), before);
        assert!(other.begin(&session, &default, &path, Kind::Bound).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    #[test]
    fn partial_native_success_views_failure_retains_live_layout_and_native_saved() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        let mut views = Views::new(&session.editor).unwrap();
        move_layout(&session, &mut views);
        native_save(&mut session, &area.path().join("model.kernspan"));
        let path = area.path().join("layout.mbse-views.json");
        let mut io = Io::default();
        let job = io.begin(&session, &views, &path, Kind::Bound).unwrap();
        fs::write(&path, b"unowned arrival").unwrap();
        let before = views.document().digest().unwrap();
        assert!(io.complete(&session, &views, execute(job)));
        assert!(io.last_error.is_some());
        assert!(!io.is_saved(&session, &views));
        assert!(!session.is_dirty());
        assert!(accepted_native(&session).is_ok());
        assert_eq!(views.document().digest().unwrap(), before);
        assert_eq!(fs::read(&path).unwrap(), b"unowned arrival");
    }
    #[test]
    fn stale_views_and_recovery_ack_cannot_preserve_new_layout() {
        for kind in [Kind::Bound, Kind::Recovery] {
            let area = area();
            let mut session = Session::new(area.path()).unwrap();
            native_save(&mut session, &area.path().join("model.kernspan"));
            let mut views = Views::new(&session.editor).unwrap();
            let mut io = Io::default();
            let path = area.path().join("layout.mbse-views.json");
            let job = io.begin(&session, &views, &path, kind.clone()).unwrap();
            let result = execute(job);
            move_layout(&session, &mut views);
            assert!(io.complete(&session, &views, result));
            assert!(!io.is_saved(&session, &views));
            assert!(!io.recovery_current(&session, &views));
            assert!(io.verify_preserved(&session, &views, kind).is_err());
        }
    }
    #[test]
    fn acknowledged_saved_and_recovery_delete_truncate_tamper_blocks_continuation() {
        for kind in [Kind::Bound, Kind::Recovery] {
            for mode in 0..3 {
                let area = area();
                let mut session = Session::new(area.path()).unwrap();
                native_save(&mut session, &area.path().join("model.kernspan"));
                let views = Views::new(&session.editor).unwrap();
                let mut io = Io::default();
                let path = area.path().join("layout.mbse-views.json");
                let job = io.begin(&session, &views, &path, kind.clone()).unwrap();
                assert!(io.complete(&session, &views, execute(job)));
                io.verify_preserved(&session, &views, kind.clone()).unwrap();
                match mode {
                    0 => fs::remove_file(&path).unwrap(),
                    1 => fs::write(&path, b"").unwrap(),
                    _ => fs::write(&path, b"tampered").unwrap(),
                }
                // Status hints may remain true; they never authorize navigation.
                assert!(io.verify_preserved(&session, &views, kind.clone()).is_err());
            }
        }
    }
    #[test]
    fn noncanonical_recovery_tracks_exact_disk_hash_and_canonical_content_separately() {
        let area = area();
        let session = Session::new(area.path()).unwrap();
        let mut views = Views::new(&session.editor).unwrap();
        let path = area.path().join("layout.mbse-views.json");
        let value: serde_json::Value =
            serde_json::from_slice(&views.recovery(&session.editor).unwrap().encoded().unwrap())
                .unwrap();
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'\n');
        fs::write(&path, &bytes).unwrap();
        let mut io = Io::default();
        io.open(&session, &mut views, &path, true).unwrap();
        assert!(io.recovery_current(&session, &views));
        io.verify_preserved(&session, &views, Kind::Recovery)
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
        assert!(io
            .verify_preserved(&session, &views, Kind::Recovery)
            .is_err());
    }
    #[test]
    fn recovered_native_document_id_refuses_old_bound_and_unbound_views() {
        let area = area();
        let native = area.path().join("model.kernspan");
        let mut session = Session::new(area.path()).unwrap();
        native_save(&mut session, &native);
        let views = Views::new(&session.editor).unwrap();
        let bound = area.path().join("bound.mbse-views.json");
        let recovery = area.path().join("recovery.mbse-views.json");
        let mut io = Io::default();
        for (path, kind) in [(&bound, Kind::Bound), (&recovery, Kind::Recovery)] {
            let job = io.begin(&session, &views, path, kind).unwrap();
            io.complete(&session, &views, execute(job));
        }
        let job = session.begin_recovery().unwrap();
        let result = stage1_io::execute_write(job);
        assert!(session.complete(result));
        let recovered = Session::restore(area.path(), session.last_recovery().unwrap()).unwrap();
        assert_ne!(
            recovered.editor.snapshot.document_id,
            session.editor.snapshot.document_id
        );
        let mut retained = Views::new(&recovered.editor).unwrap();
        let mut other = Io::default();
        assert!(other
            .open(&recovered, &mut retained, &bound, false)
            .is_err());
        assert!(other
            .open(&recovered, &mut retained, &recovery, true)
            .is_err());
        assert!(recovered.native_checkpoint().unwrap().is_none());
    }
    #[test]
    fn missing_future_unknown_duplicate_corrupt_inputs_preserve_default_and_original() {
        let area = area();
        let session = Session::new(area.path()).unwrap();
        let mut views = Views::new(&session.editor).unwrap();
        let mut io = Io::default();
        let path = area.path().join("layout.mbse-views.json");
        let original = views.document().digest().unwrap();
        assert!(io.open(&session, &mut views, &path, true).is_err());
        let base =
            String::from_utf8(views.recovery(&session.editor).unwrap().encoded().unwrap()).unwrap();
        for bytes in [
            "{".to_string(),
            base.replace(crate::native_views::RECOVERY_PROFILE, "future/100"),
            base.replacen('{', "{\"unknown\":1,", 1),
            base.replacen('{', "{\"profile\":\"duplicate\",", 1),
        ] {
            fs::write(&path, bytes.as_bytes()).unwrap();
            assert!(io.open(&session, &mut views, &path, true).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes.as_bytes());
            assert_eq!(views.document().digest().unwrap(), original);
        }
    }
    #[test]
    fn failed_native_completion_does_not_mint_checkpoint_and_old_ack_is_historical() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        let path = area.path().join("model.kernspan");
        let job = session.begin_save(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let result = stage1_io::execute_write(job);
        assert!(!result.succeeded());
        assert!(session.complete(result));
        assert!(session.native_checkpoint().unwrap().is_none());
        fs::remove_dir(&path).unwrap();
        let job = session.begin_save(&path).unwrap();
        let request = job.ticket.request_id.clone();
        let result = stage1_io::execute_write(job);
        let mut views = Views::new(&session.editor).unwrap();
        let stamp = views.stamp(&session.editor);
        views
            .dispatch(
                &mut session.editor,
                &stamp,
                NativeAction::Command(Command::EditText {
                    key: RawKey {
                        record: RecordId(1),
                        field: "name".into(),
                        item: None,
                    },
                    text: "New live edit".into(),
                }),
            )
            .unwrap();
        assert!(session.complete(result));
        let checkpoint = session.native_checkpoint().unwrap().unwrap();
        assert_eq!(checkpoint.accepted_request(), Some(&request));
        assert!(!checkpoint.is_current(&session));
        assert!(accepted_native(&session).is_err());
    }
    #[test]
    fn native_checkpoint_detects_on_disk_change_and_open_is_sealed_without_request() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        let path = area.path().join("model.kernspan");
        native_save(&mut session, &path);
        let opened = Session::open(area.path(), &path).unwrap();
        let checkpoint = opened.native_checkpoint().unwrap().unwrap();
        assert!(checkpoint.accepted_request().is_none());
        assert!(checkpoint.is_current(&opened));
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();
        assert!(opened.native_checkpoint().is_err());
    }
    #[test]
    fn native_and_views_freeze_together_and_old_start_token_cannot_clear_new_job() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        let mut views = Views::new(&session.editor).unwrap();
        let mut io = Io::default();
        let capture = views.capture(&session.editor);
        let native = session
            .begin_save(&area.path().join("model.kernspan"))
            .unwrap();
        move_layout(&session, &mut views);
        assert!(session.complete(stage1_io::execute_write(native)));
        let path = area.path().join("layout.mbse-views.json");
        let job = io
            .begin_captured(&session, capture, &path, Kind::Bound)
            .unwrap();
        let old_token = job.start_token();
        assert!(io.complete(&session, &views, execute(job)));
        assert!(!io.is_saved(&session, &views));
        let next = io.begin(&session, &views, &path, Kind::Bound).unwrap();
        let token = next.start_token();
        assert!(!io.failed_start(&session, old_token, "stale".into()));
        assert!(io.pending());
        assert!(io.failed_start(&session, token, "injected".into()));
        assert!(!io.pending());
        drop(next);
    }
    fn replacement_move(session: &Session, views: &mut Views, x: f64) {
        let id = *views.document().occurrences.keys().next().unwrap();
        let stamp = views.stamp(&session.editor);
        views
            .apply(
                &session.editor,
                &stamp,
                Layout::Move(id, Geometry::at(x, 456.)),
            )
            .unwrap();
    }
    fn independent_layout_file(session: &Session, views: &Views, path: &Path, kind: Kind) {
        let mut writer = Io::default();
        let job = writer.begin(session, views, path, kind).unwrap();
        assert!(writer.complete(session, views, execute(job)));
        assert!(writer.last_error.is_none());
    }
    #[test]
    fn replacement_clears_opposite_ack_and_revision_collision_cannot_discard_recovery_edits() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        native_save(&mut session, &area.path().join("model.kernspan"));
        let mut a = Views::new(&session.editor).unwrap();
        for x in [10., 20., 30., 40.] {
            replacement_move(&session, &mut a, x);
        }
        let a_path = area.path().join("a.mbse-views.json");
        let mut io = Io::default();
        let job = io.begin(&session, &a, &a_path, Kind::Bound).unwrap();
        io.complete(&session, &a, execute(job));
        assert_eq!(a.document().revision(), 5);
        let mut b = Views::new(&session.editor).unwrap();
        for x in [100., 200., 300.] {
            replacement_move(&session, &mut b, x);
        }
        let b_path = area.path().join("b.mbse-views.json");
        independent_layout_file(&session, &b, &b_path, Kind::Recovery);
        assert_eq!(b.document().revision(), 4);
        io.open(&session, &mut a, &b_path, true).unwrap();
        assert!(io.saved.is_none());
        assert!(io.recovery_current(&session, &a));
        replacement_move(&session, &mut a, 400.);
        assert_eq!(a.document().revision(), 5);
        assert!(!io.is_saved(&session, &a));
        let before = a.document().digest().unwrap();
        let history = a.history_usage();
        let a_bytes = fs::read(&a_path).unwrap();
        let b_bytes = fs::read(&b_path).unwrap();
        assert!(io
            .open(&session, &mut a, &a_path, false)
            .unwrap_err()
            .contains("PRESERVATION"));
        assert_eq!(a.document().digest().unwrap(), before);
        assert_eq!(a.history_usage(), history);
        assert_eq!(fs::read(&a_path).unwrap(), a_bytes);
        assert_eq!(fs::read(&b_path).unwrap(), b_bytes);
    }
    #[test]
    fn same_revision_different_bound_files_replace_ack_content_not_just_counter() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        native_save(&mut session, &area.path().join("model.kernspan"));
        let mut a = Views::new(&session.editor).unwrap();
        replacement_move(&session, &mut a, 10.);
        let mut b = Views::new(&session.editor).unwrap();
        replacement_move(&session, &mut b, 900.);
        assert_eq!(a.document().revision(), b.document().revision());
        let a_path = area.path().join("a.mbse-views.json");
        let b_path = area.path().join("b.mbse-views.json");
        let mut io = Io::default();
        let job = io.begin(&session, &a, &a_path, Kind::Bound).unwrap();
        io.complete(&session, &a, execute(job));
        independent_layout_file(&session, &b, &b_path, Kind::Bound);
        io.open(&session, &mut a, &b_path, false).unwrap();
        assert_eq!(io.saved_path(), Some(b_path.as_path()));
        assert!(io.recovered.is_none());
        io.verify_preserved(&session, &a, Kind::Bound).unwrap();
        fs::write(&b_path, b"changed").unwrap();
        let digest = a.document().digest().unwrap();
        assert!(io.open(&session, &mut a, &a_path, false).is_err());
        assert_eq!(a.document().digest().unwrap(), digest);
    }
    #[test]
    fn local_replacement_rechecks_deleted_truncated_and_tampered_bound_or_recovery_artifact() {
        for kind in [Kind::Bound, Kind::Recovery] {
            for mode in 0..3 {
                let area = area();
                let mut session = Session::new(area.path()).unwrap();
                native_save(&mut session, &area.path().join("model.kernspan"));
                let mut current = Views::new(&session.editor).unwrap();
                replacement_move(&session, &mut current, 500.);
                let current_path = area.path().join("current.mbse-views.json");
                let mut io = Io::default();
                let job = io
                    .begin(&session, &current, &current_path, kind.clone())
                    .unwrap();
                io.complete(&session, &current, execute(job));
                let target = Views::new(&session.editor).unwrap();
                let target_path = area.path().join("target.mbse-views.json");
                independent_layout_file(&session, &target, &target_path, Kind::Recovery);
                let target_bytes = fs::read(&target_path).unwrap();
                match mode {
                    0 => fs::remove_file(&current_path).unwrap(),
                    1 => fs::write(&current_path, b"").unwrap(),
                    _ => fs::write(&current_path, b"tampered").unwrap(),
                }
                let digest = current.document().digest().unwrap();
                let history = current.history_usage();
                assert!(io.open(&session, &mut current, &target_path, true).is_err());
                assert_eq!(current.document().digest().unwrap(), digest);
                assert_eq!(current.history_usage(), history);
                assert_eq!(fs::read(&target_path).unwrap(), target_bytes);
            }
        }
    }
    #[test]
    fn imported_revision_one_has_no_runtime_pristine_replacement_capability() {
        let area = area();
        let session = Session::new(area.path()).unwrap();
        let default = Views::new(&session.editor).unwrap();
        assert!(default.is_pristine_default(&session.editor));
        let bytes = default
            .recovery(&session.editor)
            .unwrap()
            .encoded()
            .unwrap();
        let mut imported = Views::restore(&bytes, &session.editor).unwrap();
        assert_eq!(imported.document().revision(), 1);
        assert!(!imported.is_pristine_default(&session.editor));
        let target_path = area.path().join("target.mbse-views.json");
        fs::write(&target_path, &bytes).unwrap();
        let mut io = Io::default();
        assert!(io
            .open(&session, &mut imported, &target_path, true)
            .is_err());
        let mut runtime_default = Views::new(&session.editor).unwrap();
        io.open(&session, &mut runtime_default, &target_path, true)
            .unwrap();
        assert!(!runtime_default.is_pristine_default(&session.editor));
        fs::remove_file(&target_path).unwrap();
        let another = area.path().join("another.mbse-views.json");
        fs::write(&another, bytes).unwrap();
        assert!(io
            .open(&session, &mut runtime_default, &another, true)
            .is_err());
    }
    #[test]
    fn recovery_to_bound_replacement_clears_recovery_ack_and_pending_refuses_all_replacement() {
        let area = area();
        let mut session = Session::new(area.path()).unwrap();
        native_save(&mut session, &area.path().join("model.kernspan"));
        let mut views = Views::new(&session.editor).unwrap();
        replacement_move(&session, &mut views, 80.);
        let mut io = Io::default();
        let recovery = area.path().join("recovery.mbse-views.json");
        let job = io
            .begin(&session, &views, &recovery, Kind::Recovery)
            .unwrap();
        io.complete(&session, &views, execute(job));
        let bound = area.path().join("bound.mbse-views.json");
        independent_layout_file(&session, &views, &bound, Kind::Bound);
        io.open(&session, &mut views, &bound, false).unwrap();
        assert!(io.recovered.is_none());
        assert!(io.is_saved(&session, &views));
        let pending = io.begin(&session, &views, &bound, Kind::Bound).unwrap();
        let before = views.document().digest().unwrap();
        assert!(io.open(&session, &mut views, &recovery, true).is_err());
        assert_eq!(views.document().digest().unwrap(), before);
        assert!(io.pending());
        drop(pending);
    }
}
