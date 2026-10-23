//! Bounded local Stage1 snapshot storage and explicit restricted SysML copies.
//!
//! A write freezes accepted records and raw/creation journals together. Only a
//! receipt for that exact active request can acknowledge it; live edits remain
//! dirty when an older frozen write completes. Recovery is a separate durable
//! copy and never acknowledges the native destination as saved.
use crate::stage1::{DocumentId, Editor, RecordKind, Snapshot};
use crate::stage1_sysml as exchange;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const PROFILE: &str = "kernspan.stage1/2";
const LEGACY_PROFILE: &str = "kernspan.stage1/1";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    Unsaved,
    Saving,
    Saved,
    Error(String),
    Conflict(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteKind {
    Save,
    Recovery,
}

/// Runtime identities have distinct types and roles. The retained filesystem
/// reservation makes their nonce unavailable to a later process's allocator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstanceId(pub String);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub u64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoTicket {
    pub document_id: DocumentId,
    pub instance_id: InstanceId,
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub model_revision: u64,
    pub raw_version: u64,
    pub destination: PathBuf,
    pub snapshot_hash: String,
}

#[derive(Clone, Debug)]
pub struct WriteJob {
    pub ticket: IoTicket,
    pub snapshot: Snapshot,
    pub kind: WriteKind,
    // Private immutable encodings and ownership stop callers from turning a
    // changed public snapshot/ticket into a receipt for different frozen bytes.
    bytes: Vec<u8>,
    frozen_ticket: IoTicket,
    frozen_kind: WriteKind,
    expected_disk_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteFailure {
    pub message: String,
    pub conflict: bool,
}
impl fmt::Display for WriteFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
// Seal construction to this module; non_exhaustive would also allow sibling
// modules to construct a receipt without actual I/O.
#[allow(clippy::manual_non_exhaustive)]
pub struct WriteReceipt {
    pub destination: PathBuf,
    pub envelope_hash: String,
    pub snapshot_hash: String,
    pub written_bytes: u64,
    _verified_write: (),
}

#[derive(Clone, Debug)]
pub struct WriteResult {
    pub ticket: IoTicket,
    pub kind: WriteKind,
    outcome: Result<WriteReceipt, WriteFailure>,
    executed_ticket: IoTicket,
    executed_kind: WriteKind,
    frozen_hash: String,
}
impl WriteResult {
    pub fn outcome(&self) -> Result<&WriteReceipt, &WriteFailure> {
        self.outcome.as_ref()
    }
    pub fn succeeded(&self) -> bool {
        self.outcome.is_ok()
    }
    pub fn error(&self) -> Option<&WriteFailure> {
        self.outcome.as_ref().err()
    }
}

/// Bounded exact refused input retained by a desktop caller independently of a
/// mutable filesystem path. I/O failures before reading have no byte asset.
#[derive(Clone, Debug)]
pub struct ImportRefusal {
    pub message: String,
    pub source_asset: Option<exchange::SourceAsset>,
}
/// A SysML copy has its own operation and durable receipt. It cannot acknowledge
/// a native Save, recovery or lifecycle continuation.
#[derive(Clone, Debug)]
pub struct ExportJob {
    pub ticket: IoTicket,
    bytes: Vec<u8>,
    frozen_ticket: IoTicket,
    semantic_digest: String,
    excluded_state: Vec<exchange::ExcludedState>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExportReceipt {
    pub ticket: IoTicket,
    pub source_sha256: String,
    pub semantic_digest: String,
    pub written_bytes: u64,
    pub excluded_state: Vec<exchange::ExcludedState>,
}
#[derive(Clone, Debug)]
pub struct ExportResult {
    ticket: IoTicket,
    outcome: Result<ExportReceipt, WriteFailure>,
}
impl ExportResult {
    pub fn outcome(&self) -> Result<&ExportReceipt, &WriteFailure> {
        self.outcome.as_ref()
    }
}

/// Retain every original source revision rather than reducing provenance to
/// paths. The same path can legitimately appear with several historical hashes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedSource {
    path: PathBuf,
    hash: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    source_path: Option<PathBuf>,
    source_hash: Option<String>,
    protected_sources: Vec<ProtectedSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    profile: String,
    schema_version: u32,
    kind: WriteKind,
    snapshot: Snapshot,
    provenance: Provenance,
}
#[derive(Clone, Debug)]
struct Pending {
    ticket: IoTicket,
    kind: WriteKind,
    envelope_hash: String,
    byte_len: u64,
}

/// Proof of an accepted native Save/open. Only Session can construct it.
/// It may describe an older durable snapshot while the live editor is dirty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCheckpoint {
    document_id: DocumentId,
    artifact_sha256: String,
    snapshot_sha256: String,
    path: PathBuf,
    session_id: SessionId,
    instance_id: InstanceId,
    accepted_request: Option<RequestId>,
}
impl NativeCheckpoint {
    pub fn document_id(&self) -> &DocumentId { &self.document_id }
    pub fn artifact_sha256(&self) -> &str { &self.artifact_sha256 }
    pub fn snapshot_sha256(&self) -> &str { &self.snapshot_sha256 }
    pub fn path(&self) -> &Path { &self.path }
    pub fn session_id(&self) -> &SessionId { &self.session_id }
    pub fn instance_id(&self) -> &InstanceId { &self.instance_id }
    pub fn accepted_request(&self) -> Option<&RequestId> { self.accepted_request.as_ref() }
    pub fn is_current(&self, session: &Session) -> bool {
        self.session_id == session.session_id && self.instance_id == session.instance_id
            && self.document_id == session.editor.snapshot.document_id
            && self.snapshot_sha256 == session.editor.snapshot.digest()
            && session.disk_hash.as_deref() == Some(self.artifact_sha256.as_str())
    }
}

pub struct Session {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub state_root: PathBuf,
    session_id: SessionId,
    instance_id: InstanceId,
    reservation: PathBuf,
    next_request: u64,
    pending: Option<Pending>,
    durable_snapshot_hash: Option<String>,
    accepted_save_request: Option<RequestId>,
    disk_hash: Option<String>,
    last_failure: Option<WriteFailure>,
    recovery_path: Option<PathBuf>,
    recovery_disk_hash: Option<String>,
    recovery_snapshot_hash: Option<String>,
    protected_sources: BTreeSet<ProtectedSource>,
    autosave: bool,
    pending_export: Option<(IoTicket, String)>,
    last_export: Option<ExportReceipt>,
}
impl Session {
    /// Status hint only: accepted artifact identity without filesystem I/O.
    /// Destructive navigation must use native_checkpoint() for revalidation.
    pub fn accepted_native_artifact_sha256(&self) -> Option<&str> { self.disk_hash.as_deref() }
    /// Revalidate the accepted file, never infer success from complete()'s bool.
    /// Recovery and failed writes cannot mint a native checkpoint.
    pub fn native_checkpoint(&self) -> Result<Option<NativeCheckpoint>, String> {
        let (Some(path), Some(disk), Some(snapshot)) =
            (&self.path, &self.disk_hash, &self.durable_snapshot_hash) else { return Ok(None); };
        let bytes = read_bounded(path)?;
        if hash(&bytes) != *disk { return Err("STAGE1_CHECKPOINT_DISK_CHANGED".into()); }
        let envelope = decode(&bytes)?;
        if envelope.kind != WriteKind::Save || envelope.snapshot.digest() != *snapshot
            || envelope.snapshot.document_id != self.editor.snapshot.document_id {
            return Err("STAGE1_CHECKPOINT_IDENTITY_CHANGED".into());
        }
        Ok(Some(NativeCheckpoint {
            document_id: envelope.snapshot.document_id,
            artifact_sha256: disk.clone(), snapshot_sha256: snapshot.clone(), path: path.clone(),
            session_id: self.session_id.clone(), instance_id: self.instance_id.clone(),
            accepted_request: self.accepted_save_request.clone(),
        }))
    }

    pub fn new(state_root: &Path) -> Result<Self, String> {
        let (root, reservation, session, instance) = reserve(state_root)?;
        let snapshot = Snapshot::new(DocumentId(format!("document:{}", session.0)));
        Self::assemble(root, reservation, session, instance, snapshot)
    }

    pub fn from_fixture(state_root: &Path, fixture: &Value) -> Result<Self, String> {
        let (root, reservation, session, instance) = reserve(state_root)?;
        let snapshot =
            Snapshot::from_fixture(DocumentId(format!("illustrative:{}", session.0)), fixture)?;
        Self::assemble(root, reservation, session, instance, snapshot)
    }

    pub fn open(state_root: &Path, path: &Path) -> Result<Self, String> {
        let path = native_path(path)?;
        let bytes = read_bounded(&path)?;
        let envelope = decode(&bytes)?;
        if envelope.kind != WriteKind::Save {
            return Err(
                "STAGE1_RECOVERY_REQUIRES_RESTORE: open recovery as an independent draft".into(),
            );
        }
        let (root, reservation, session, instance) = reserve(state_root)?;
        let mut result = Self::assemble(root, reservation, session, instance, envelope.snapshot)?;
        result.durable_snapshot_hash = Some(result.editor.snapshot.digest());
        result.disk_hash = Some(hash(&bytes));
        result.path = Some(path);
        result
            .protected_sources
            .extend(envelope.provenance.protected_sources);
        Ok(result)
    }

    /// Parse first, then adopt all supported meaning into a fresh dirty session.
    /// The exact original remains a protected source, never a Save destination.
    pub fn import_sysml(state_root: &Path, path: &Path) -> Result<Self, String> {
        Self::import_sysml_retained(state_root, path).map_err(|failure| failure.message)
    }
    pub fn import_sysml_retained(state_root: &Path, path: &Path) -> Result<Self, ImportRefusal> {
        let path = exchange_path(path).map_err(|message| ImportRefusal {
            message,
            source_asset: None,
        })?;
        let bytes = read_bounded(&path).map_err(|message| ImportRefusal {
            message,
            source_asset: None,
        })?;
        let outcome = (|| -> Result<Self, String> {
            let logical_key = "imported-source.sysml";
            let candidate = exchange::import(logical_key, &bytes).map_err(|failure| {
                serde_json::to_string(&failure.diagnostics)
                    .unwrap_or_else(|_| "STAGE1_SYSML_IMPORT".into())
            })?;
            let (root, reservation, session, instance) = reserve(state_root)?;
            let mut snapshot = Snapshot::new(DocumentId(format!("imported:{}", session.0)));
            snapshot.records = candidate.semantics.records;
            let system = snapshot
                .records
                .values()
                .find(|record| record.kind == RecordKind::System)
                .map(|record| record.id)
                .ok_or("STAGE1_SYSML_SYSTEM_REQUIRED")?;
            snapshot
                .records
                .get_mut(&system)
                .expect("found System")
                .fields
                .insert(
                    "_kernspanSysmlSource".into(),
                    serde_json::to_value(candidate.source).map_err(|e| e.to_string())?,
                );
            let max = snapshot
                .records
                .values()
                .flat_map(|record| {
                    std::iter::once(record.id.0).chain(
                        record
                            .fields
                            .get("context")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(|context| context.get("id").and_then(Value::as_u64)),
                    )
                })
                .max()
                .unwrap_or(0);
            snapshot.next_id = 1000.max(max.checked_add(1).ok_or("STAGE1_ID_EXHAUSTED")?);
            snapshot.validate()?;
            let mut result = Self::assemble(root, reservation, session, instance, snapshot)?;
            result.protected_sources.insert(ProtectedSource {
                path,
                hash: hash(&bytes),
            });
            Ok(result)
        })();
        outcome.map_err(|message| ImportRefusal {
            message,
            source_asset: Some(exchange::SourceAsset {
                logical_key: "imported-source.sysml".into(),
                original_bytes: (bytes.len() <= crate::boundary::MAX_SOURCE).then(|| bytes.clone()),
                byte_len: bytes.len(),
                sha256: hash(&bytes),
            }),
        })
    }

    pub fn export_pending(&self) -> bool {
        self.pending_export.is_some()
    }
    pub fn last_export(&self) -> Option<&ExportReceipt> {
        self.last_export.as_ref()
    }
    pub fn begin_export_sysml(&mut self, path: &Path) -> Result<ExportJob, String> {
        if self.export_pending() {
            return Err("STAGE1_EXPORT_PENDING".into());
        }
        self.editor.snapshot.validate()?;
        let destination = exchange_path(path)?;
        if self
            .protected_sources
            .iter()
            .any(|source| source.path == destination)
        {
            return Err("STAGE1_PROTECTED_SOURCE: choose a new SysML copy".into());
        }
        check_destination(&destination, None).map_err(|e| e.to_string())?;
        let model =
            exchange::from_records(&self.editor.snapshot.records).map_err(|e| format!("{e:?}"))?;
        let candidate = exchange::export(&model).map_err(|e| format!("{e:?}"))?;
        let next = self
            .next_request
            .checked_add(1)
            .ok_or("STAGE1_REQUEST_ID_EXHAUSTED")?;
        let ticket = IoTicket {
            document_id: self.editor.snapshot.document_id.clone(),
            instance_id: self.instance_id.clone(),
            session_id: self.session_id.clone(),
            request_id: RequestId(self.next_request),
            model_revision: self.editor.snapshot.model_revision,
            raw_version: self.editor.snapshot.raw_version,
            destination,
            snapshot_hash: self.editor.snapshot.digest(),
        };
        self.next_request = next;
        self.pending_export = Some((ticket.clone(), candidate.sha256));
        Ok(ExportJob {
            frozen_ticket: ticket.clone(),
            ticket,
            bytes: candidate.utf8.into_bytes(),
            semantic_digest: candidate.semantic_digest,
            excluded_state: candidate.excluded_session_state,
        })
    }
    /// Consume only this export; native write state and durability stay untouched.
    pub fn complete_export(&mut self, result: ExportResult) -> Result<bool, String> {
        let Some((ticket, expected_hash)) = self.pending_export.as_ref() else {
            return Ok(false);
        };
        if ticket != &result.ticket {
            return Ok(false);
        }
        if let Ok(receipt) = &result.outcome {
            if &receipt.ticket != ticket || &receipt.source_sha256 != expected_hash {
                return Ok(false);
            }
        }
        self.pending_export = None;
        let receipt = result.outcome.map_err(|e| e.to_string())?;
        let bytes = read_bounded(&receipt.ticket.destination)?;
        if hash(&bytes) != receipt.source_sha256 || bytes.len() as u64 != receipt.written_bytes {
            return Err("STAGE1_EXPORT_RECEIPT_DISK_CONFLICT".into());
        }
        self.last_export = Some(receipt);
        Ok(true)
    }

    /// Restoring never makes the protected source the active destination. A new
    /// semantic document ID and runtime instance reset interactive history while
    /// all record IDs and raw/creation journals remain intact.
    pub fn restore(state_root: &Path, path: &Path) -> Result<Self, String> {
        let path = native_path(path)?;
        let bytes = read_bounded(&path)?;
        let mut envelope = decode(&bytes)?;
        let (root, reservation, session, instance) = reserve(state_root)?;
        envelope.snapshot.document_id = DocumentId(format!("recovered:{}", session.0));
        let mut result = Self::assemble(root, reservation, session, instance, envelope.snapshot)?;
        result.protected_sources.insert(ProtectedSource {
            path,
            hash: hash(&bytes),
        });
        result
            .protected_sources
            .extend(envelope.provenance.protected_sources);
        if let (Some(path), Some(hash)) = (
            envelope.provenance.source_path,
            envelope.provenance.source_hash,
        ) {
            result
                .protected_sources
                .insert(ProtectedSource { path, hash });
        }
        Ok(result)
    }

    fn assemble(
        state_root: PathBuf,
        reservation: PathBuf,
        session_id: SessionId,
        instance_id: InstanceId,
        snapshot: Snapshot,
    ) -> Result<Self, String> {
        let editor = Editor::new(snapshot, instance_id.0.clone())?;
        Ok(Self {
            editor,
            path: None,
            state_root,
            session_id,
            instance_id,
            reservation,
            next_request: 1,
            pending: None,
            durable_snapshot_hash: None,
            accepted_save_request: None,
            disk_hash: None,
            last_failure: None,
            recovery_path: None,
            recovery_disk_hash: None,
            recovery_snapshot_hash: None,
            protected_sources: BTreeSet::new(),
            autosave: false,
            pending_export: None,
            last_export: None,
        })
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }
    pub fn pending_ticket(&self) -> Option<&IoTicket> {
        self.pending.as_ref().map(|pending| &pending.ticket)
    }
    /// Kind belongs to the same active ticket; UI continuations must capture it.
    pub fn pending_kind(&self) -> Option<WriteKind> {
        self.pending.as_ref().map(|pending| pending.kind.clone())
    }

    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn is_dirty(&self) -> bool {
        match &self.durable_snapshot_hash {
            None => true,
            Some(saved) => saved != &self.editor.snapshot.digest(),
        }
    }
    pub fn status(&self) -> SaveStatus {
        if self.pending() {
            return SaveStatus::Saving;
        }
        if let Some(failure) = &self.last_failure {
            return if failure.conflict {
                SaveStatus::Conflict(failure.message.clone())
            } else {
                SaveStatus::Error(failure.message.clone())
            };
        }
        if self.is_dirty() {
            SaveStatus::Unsaved
        } else {
            SaveStatus::Saved
        }
    }
    pub fn last_recovery(&self) -> Option<&Path> {
        self.recovery_path.as_deref()
    }
    pub fn recovery_current(&self) -> bool {
        // Lifecycle callers recheck the actual acknowledged recovery bytes so a
        // removed or truncated recovery can never authorize dropping live input.
        // Runtime IDs cannot change inside this Session, and a new Session has
        // no successful recovery receipt to compare.
        let Some(path) = &self.recovery_path else {
            return false;
        };
        self.recovery_snapshot_hash.as_ref() == Some(&self.editor.snapshot.digest())
            && read_bounded(path)
                .ok()
                .is_some_and(|bytes| self.recovery_disk_hash.as_ref() == Some(&hash(&bytes)))
    }
    pub fn set_autosave(&mut self, enabled: bool) {
        self.autosave = enabled;
    }
    pub fn autosave(&self) -> bool {
        self.autosave
    }

    /// The caller owns a bounded quiet-period debounce. This method invents
    /// neither a destination nor a timer, and toggling preference never writes.
    pub fn begin_autosave(&mut self) -> Result<Option<WriteJob>, String> {
        if !self.autosave || self.pending() || !self.is_dirty() {
            return Ok(None);
        }
        let Some(path) = self.path.clone() else {
            return Ok(None);
        };
        self.begin_save(&path).map(Some)
    }

    pub fn begin_save(&mut self, path: &Path) -> Result<WriteJob, String> {
        let result = self.prepare_save(path);
        self.note_begin_error(&result);
        result
    }
    fn prepare_save(&mut self, path: &Path) -> Result<WriteJob, String> {
        if self.pending() {
            return Err("STAGE1_WRITE_PENDING".into());
        }
        let destination = native_path(path)?;
        if self
            .protected_sources
            .iter()
            .any(|source| source.path == destination)
        {
            return Err(
                "STAGE1_PROTECTED_SOURCE: save the independent copy to another path".into(),
            );
        }
        let expected = if self.path.as_ref() == Some(&destination) {
            self.disk_hash.clone()
        } else {
            None
        };
        // Refuse an existing Save As target even when it is another recognized
        // document: ownership of this destination was never established.
        check_destination(&destination, expected.as_deref()).map_err(|error| error.message)?;
        self.freeze(WriteKind::Save, destination, expected)
    }
    pub fn begin_recovery(&mut self) -> Result<WriteJob, String> {
        let result = if self.pending() {
            Err("STAGE1_WRITE_PENDING".into())
        } else {
            let destination = self.reservation.join("recovery.kernspan");
            self.freeze(
                WriteKind::Recovery,
                destination,
                self.recovery_disk_hash.clone(),
            )
        };
        self.note_begin_error(&result);
        result
    }
    fn note_begin_error(&mut self, result: &Result<WriteJob, String>) {
        if let Err(message) = result {
            if !self.pending() {
                self.last_failure = Some(WriteFailure {
                    conflict: message.contains("CONFLICT")
                        || message.contains("EXISTING")
                        || message.contains("PROTECTED")
                        || message.contains("DISAPPEARED"),
                    message: message.clone(),
                });
            }
        }
    }
    fn freeze(
        &mut self,
        kind: WriteKind,
        destination: PathBuf,
        expected_disk_hash: Option<String>,
    ) -> Result<WriteJob, String> {
        self.editor.snapshot.validate()?;
        let next = self
            .next_request
            .checked_add(1)
            .ok_or("STAGE1_REQUEST_ID_EXHAUSTED")?;
        let snapshot = self.editor.snapshot.clone();
        let ticket = IoTicket {
            document_id: snapshot.document_id.clone(),
            instance_id: self.instance_id.clone(),
            session_id: self.session_id.clone(),
            request_id: RequestId(self.next_request),
            model_revision: snapshot.model_revision,
            raw_version: snapshot.raw_version,
            destination,
            snapshot_hash: snapshot.digest(),
        };
        let provenance = Provenance {
            source_path: if kind == WriteKind::Recovery {
                self.path.clone()
            } else {
                None
            },
            source_hash: if kind == WriteKind::Recovery {
                self.disk_hash.clone()
            } else {
                None
            },
            protected_sources: self.protected_sources.iter().cloned().collect(),
        };
        let envelope = Envelope {
            profile: PROFILE.into(),
            schema_version: SCHEMA_VERSION,
            kind: kind.clone(),
            snapshot: snapshot.clone(),
            provenance,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("STAGE1_DOCUMENT_LIMIT".into());
        }
        self.pending = Some(Pending {
            ticket: ticket.clone(),
            kind: kind.clone(),
            envelope_hash: hash(&bytes),
            byte_len: bytes.len() as u64,
        });
        self.next_request = next;
        self.last_failure = None;
        Ok(WriteJob {
            frozen_ticket: ticket.clone(),
            frozen_kind: kind.clone(),
            ticket,
            snapshot,
            kind,
            bytes,
            expected_disk_hash,
        })
    }

    /// True means this active operation was consumed, including a genuine I/O
    /// failure. It is not a success flag. Mismatched, duplicate or forged-looking
    /// completions leave the active request owned and untouched.
    pub fn complete(&mut self, result: WriteResult) -> bool {
        let Some(pending) = self.pending.as_ref() else {
            return false;
        };
        if pending.ticket != result.ticket
            || pending.kind != result.kind
            || result.executed_ticket != result.ticket
            || result.executed_kind != result.kind
            || result.frozen_hash != pending.envelope_hash
        {
            return false;
        }
        if let Ok(receipt) = &result.outcome {
            if receipt.destination != pending.ticket.destination
                || receipt.snapshot_hash != pending.ticket.snapshot_hash
                || receipt.envelope_hash != pending.envelope_hash
                || receipt.written_bytes != pending.byte_len
            {
                return false;
            }
        }
        let pending = self.pending.take().expect("checked active request");
        match result.outcome {
            Err(failure) => {
                self.last_failure = Some(failure);
            }
            Ok(receipt) => {
                // Do not announce Saved if the file disappeared or was changed
                // between worker completion and processing the receipt.
                match read_bounded(&receipt.destination) {
                    Ok(bytes) if hash(&bytes) == receipt.envelope_hash => {
                        self.last_failure = None;
                        match pending.kind {
                            WriteKind::Save => {
                                self.accepted_save_request = Some(pending.ticket.request_id.clone());
                                self.path = Some(receipt.destination);
                                self.disk_hash = Some(receipt.envelope_hash);
                                self.durable_snapshot_hash = Some(pending.ticket.snapshot_hash);
                            }
                            WriteKind::Recovery => {
                                self.recovery_path = Some(receipt.destination);
                                self.recovery_disk_hash = Some(receipt.envelope_hash);
                                self.recovery_snapshot_hash = Some(pending.ticket.snapshot_hash);
                            }
                        }
                    }
                    _ => self.last_failure = Some(conflict("STAGE1_RECEIPT_DISK_CONFLICT")),
                }
            }
        }
        true
    }
}

fn reserve(state_root: &Path) -> Result<(PathBuf, PathBuf, SessionId, InstanceId), String> {
    fs::create_dir_all(state_root).map_err(|error| format!("STAGE1_STATE_ROOT: {error}"))?;
    let metadata = fs::symlink_metadata(state_root).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_dir() {
        return Err("STAGE1_UNSAFE_STATE_ROOT".into());
    }
    let root = fs::canonicalize(state_root).map_err(|error| error.to_string())?;
    let reserved = tempfile::Builder::new()
        .prefix("stage1-session-")
        .tempdir_in(&root)
        .map_err(|error| error.to_string())?
        .keep();
    let nonce = reserved
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("STAGE1_RUNTIME_ID")?;
    let session = SessionId(format!("session:{nonce}"));
    let instance = InstanceId(format!("instance:{nonce}"));
    Ok((root, reserved, session, instance))
}

fn exchange_path(path: &Path) -> Result<PathBuf, String> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("sysml") {
        return Err("STAGE1_SYSML_EXTENSION: use .sysml for the supported exchange profile".into());
    }
    copy_path(path)
}
fn native_path(path: &Path) -> Result<PathBuf, String> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("kernspan") {
        return Err(
            "STAGE1_NATIVE_EXTENSION: use .kernspan for the app-owned Stage1 profile".into(),
        );
    }
    copy_path(path)
}
fn copy_path(path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("STAGE1_DESTINATION_NAME")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|error| format!("STAGE1_PARENT: {error}"))?;
    if !fs::metadata(&parent)
        .map_err(|error| error.to_string())?
        .is_dir()
    {
        return Err("STAGE1_PARENT_DIRECTORY".into());
    }
    Ok(parent.join(name))
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn conflict(message: &str) -> WriteFailure {
    WriteFailure {
        message: message.into(),
        conflict: true,
    }
}
fn io_failure(error: impl fmt::Display) -> WriteFailure {
    WriteFailure {
        message: format!("STAGE1_IO: {error}"),
        conflict: false,
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_file() {
        return Err("STAGE1_UNSAFE_SOURCE".into());
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err("STAGE1_DOCUMENT_LIMIT".into());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(|error| error.to_string())?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("STAGE1_DOCUMENT_LIMIT".into());
    }
    Ok(bytes)
}

pub fn read_snapshot(path: &Path) -> Result<Snapshot, String> {
    let path = native_path(path)?;
    Ok(decode(&read_bounded(&path)?)?.snapshot)
}
/// Private shared admission path; recovery inventory never trusts plain JSON metadata.
pub(crate) fn decode_recovery(bytes: &[u8]) -> Result<Snapshot, String> {
    let envelope = decode(bytes)?;
    if envelope.kind != WriteKind::Recovery {
        return Err("STAGE1_NOT_RECOVERY".into());
    }
    Ok(envelope.snapshot)
}

fn decode(bytes: &[u8]) -> Result<Envelope, String> {
    if bytes.len() > MAX_BYTES {
        return Err("STAGE1_DOCUMENT_LIMIT".into());
    }
    reject_numeric_underflow(bytes)?;
    // serde_json::Value alone accepts duplicate object keys by keeping the last.
    // The recursive visitor rejects duplicates before any typed interpretation.
    let value: UniqueValue =
        serde_json::from_slice(bytes).map_err(|error| format!("STAGE1_JSON: {error}"))?;
    check_shape(&value.0)?;
    let envelope: Envelope =
        serde_json::from_value(value.0).map_err(|error| format!("STAGE1_ENVELOPE: {error}"))?;
    let legacy = envelope.profile == LEGACY_PROFILE && envelope.schema_version == 1;
    if !legacy && (envelope.profile != PROFILE || envelope.schema_version != SCHEMA_VERSION) {
        return Err("STAGE1_UNSUPPORTED_PROFILE_OR_VERSION".into());
    }
    if legacy
        && (envelope.snapshot.records.values().any(|record| {
            record.fields.contains_key("calculation")
                || record.fields.contains_key("_kernspanSysmlSource")
                || record.fields.contains_key("_kernspanCalculationRawOwners")
        }) || envelope
            .snapshot
            .creation_drafts
            .keys()
            .any(|id| id.starts_with("calc-config:"))
            || envelope
                .snapshot
                .drafts
                .iter()
                .any(|draft| draft.key.field.starts_with("calc-config:")))
    {
        return Err(
            "STAGE1_V1_TYPED_KEY_COLLISION: original opaque data retained; migration refused"
                .into(),
        );
    }
    envelope.snapshot.validate()?;
    if envelope.provenance.source_path.is_some() != envelope.provenance.source_hash.is_some() {
        return Err("STAGE1_SOURCE_PROVENANCE_PAIR_REQUIRED".into());
    }
    let valid_hash =
        |hash: &str| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
    if envelope
        .provenance
        .source_hash
        .as_deref()
        .is_some_and(|hash| !valid_hash(hash))
        || envelope
            .provenance
            .protected_sources
            .iter()
            .any(|source| !valid_hash(&source.hash))
    {
        return Err("STAGE1_SOURCE_PROVENANCE_HASH".into());
    }
    if envelope.snapshot.records.len() > 65_536
        || envelope.snapshot.drafts.len() > 65_536
        || envelope.snapshot.creation_drafts.len() > 65_536
        || envelope.provenance.protected_sources.len() > 65_536
    {
        return Err("STAGE1_COLLECTION_LIMIT".into());
    }
    Ok(envelope)
}

// Reject nonzero JSON numbers that would silently deserialize as zero. This
// preflight does not interpret strings or replace serde's JSON syntax checker.
fn reject_numeric_underflow(bytes: &[u8]) -> Result<(), String> {
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset] == b'"' {
            offset += 1;
            while offset < bytes.len() {
                match bytes[offset] {
                    b'\\' => offset = (offset + 2).min(bytes.len()),
                    b'"' => {
                        offset += 1;
                        break;
                    }
                    _ => offset += 1,
                }
            }
        } else if bytes[offset] == b'-' || bytes[offset].is_ascii_digit() {
            let start = offset;
            while offset < bytes.len()
                && matches!(
                    bytes[offset],
                    b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
                )
            {
                offset += 1;
            }
            let token =
                std::str::from_utf8(&bytes[start..offset]).map_err(|_| "STAGE1_JSON_UTF8")?;
            let mantissa = token.split(['e', 'E']).next().unwrap_or("");
            if mantissa.bytes().any(|byte| matches!(byte, b'1'..=b'9'))
                && token.parse::<f64>().is_ok_and(|number| number == 0.0)
            {
                return Err("STAGE1_UNSUPPORTED_NUMERIC_RANGE: nonzero JSON number underflows the native numeric representation".into());
            }
        } else {
            offset += 1;
        }
    }
    Ok(())
}

// Unknown data belongs in Record.fields, where it is retained verbatim as JSON
// values. Refuse unknown structural keys rather than silently dropping them at
// deserialize time. Numeric aliases such as "01" must not collapse record IDs.
fn check_shape(value: &Value) -> Result<(), String> {
    fn keys(value: &Value, allowed: &[&str], label: &str) -> Result<(), String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("STAGE1_SHAPE: {label} must be object"))?;
        if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
            return Err(format!("STAGE1_UNKNOWN_FIELD: {label}.{key}"));
        }
        Ok(())
    }
    let snapshot = value.get("snapshot").ok_or("STAGE1_MISSING_SNAPSHOT")?;
    keys(
        snapshot,
        &[
            "document_id",
            "records",
            "drafts",
            "creation_drafts",
            "next_id",
            "model_revision",
            "raw_version",
        ],
        "snapshot",
    )?;
    if let Some(records) = snapshot.get("records").and_then(Value::as_object) {
        for (key, record) in records {
            if key
                .parse::<u64>()
                .ok()
                .is_none_or(|id| id.to_string() != *key)
            {
                return Err("STAGE1_NONCANONICAL_RECORD_KEY".into());
            }
            keys(record, &["id", "kind", "name", "scope", "fields"], "record")?;
        }
    }
    if let Some(drafts) = snapshot.get("drafts").and_then(Value::as_array) {
        for draft in drafts {
            keys(
                draft,
                &[
                    "key",
                    "text",
                    "version",
                    "validity",
                    "unit_context",
                    "owner_generation",
                ],
                "draft",
            )?;
            if let Some(key) = draft.get("key") {
                keys(key, &["record", "field", "item"], "raw_key")?;
            }
        }
    }
    if let Some(drafts) = snapshot.get("creation_drafts").and_then(Value::as_object) {
        for draft in drafts.values() {
            keys(draft, &["owner", "kind", "fields"], "creation_draft")?;
        }
    }
    Ok(())
}

pub(crate) struct UniqueValue(pub(crate) Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> de::Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(Value::Number)
                    .map(UniqueValue)
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.into())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = access.next_entry::<String, UniqueValue>()? {
                    if values.insert(key.clone(), value.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn check_destination(path: &Path, expected: Option<&str>) -> Result<Option<Vec<u8>>, WriteFailure> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(conflict("STAGE1_UNSAFE_TARGET"));
            }
            let Some(expected) = expected else {
                return Err(conflict("STAGE1_EXISTING_TARGET"));
            };
            let bytes = read_bounded(path).map_err(io_failure)?;
            if hash(&bytes) != expected {
                return Err(conflict("STAGE1_DISK_CONFLICT"));
            }
            // A matching hash is ownership evidence, not permission to replace
            // a now-unrecognized profile. Always decode the prior source too.
            decode(&bytes).map_err(io_failure)?;
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if expected.is_some() {
                Err(conflict("STAGE1_SOURCE_DISAPPEARED"))
            } else {
                Ok(None)
            }
        }
        Err(error) => Err(io_failure(error)),
    }
}

struct CooperativeLock(PathBuf);
impl Drop for CooperativeLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn retain_prior(parent: &Path, prior: &[u8]) -> Result<(), WriteFailure> {
    let history = parent.join(".kernspan-stage1-history");
    match fs::create_dir(&history) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(io_failure(error)),
    }
    if !fs::symlink_metadata(&history)
        .map_err(io_failure)?
        .file_type()
        .is_dir()
    {
        return Err(conflict("STAGE1_UNSAFE_HISTORY"));
    }
    let backup = history.join(format!("{}.kernspan", hash(prior)));
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
    {
        Ok(mut file) => {
            file.write_all(prior).map_err(io_failure)?;
            file.sync_all().map_err(io_failure)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_bounded(&backup).map_err(io_failure)? != prior {
                return Err(conflict("STAGE1_HISTORY_CONFLICT"));
            }
        }
        Err(error) => return Err(io_failure(error)),
    }
    sync_parent(&history)?;
    Ok(())
}

// Directory fsync is supported by Unix. Windows tempfile's atomic replace uses
// the OS implementation; std does not offer an equivalent directory flush API.
#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<(), WriteFailure> {
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(io_failure)
}
#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<(), WriteFailure> {
    Ok(())
}

/// Publish only a new SysML file, then verify its actual committed bytes.
pub fn execute_export(job: ExportJob) -> ExportResult {
    let result = (|| {
        if job.ticket != job.frozen_ticket {
            return Err(io_failure("STAGE1_FROZEN_EXPORT_CHANGED"));
        }
        let path = &job.ticket.destination;
        let parent = path.parent().ok_or_else(|| io_failure("STAGE1_PARENT"))?;
        check_destination(path, None)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(io_failure)?;
        temp.write_all(&job.bytes).map_err(io_failure)?;
        temp.as_file().sync_all().map_err(io_failure)?;
        temp.persist_noclobber(path).map_err(|e| {
            if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                conflict("STAGE1_EXPORT_EXISTING_DESTINATION")
            } else {
                io_failure(e)
            }
        })?;
        sync_parent(parent)?;
        let actual = read_bounded(path).map_err(io_failure)?;
        if actual != job.bytes {
            return Err(conflict("STAGE1_EXPORT_POST_WRITE_CONFLICT"));
        }
        Ok(ExportReceipt {
            ticket: job.frozen_ticket.clone(),
            source_sha256: hash(&actual),
            semantic_digest: job.semantic_digest,
            written_bytes: actual.len() as u64,
            excluded_state: job.excluded_state,
        })
    })();
    ExportResult {
        ticket: job.frozen_ticket,
        outcome: result,
    }
}

/// Execute once on a worker. Cooperative writers serialize using a create-new
/// lock; external changes are checked again immediately before atomic commit.
/// An external writer that ignores the lock can still race the final OS rename;
/// this protocol does not claim a cross-process filesystem compare-and-swap.
pub fn execute_write(job: WriteJob) -> WriteResult {
    let frozen_hash = hash(&job.bytes);
    let outcome = if job.ticket != job.frozen_ticket
        || job.kind != job.frozen_kind
        || job.snapshot.digest() != job.ticket.snapshot_hash
    {
        Err(io_failure("STAGE1_FROZEN_JOB_CHANGED"))
    } else {
        write_actual(&job)
    };
    WriteResult {
        ticket: job.ticket,
        kind: job.kind,
        outcome,
        executed_ticket: job.frozen_ticket,
        executed_kind: job.frozen_kind,
        frozen_hash,
    }
}
fn write_actual(job: &WriteJob) -> Result<WriteReceipt, WriteFailure> {
    let path = &job.ticket.destination;
    let parent = path.parent().ok_or_else(|| io_failure("STAGE1_PARENT"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io_failure("STAGE1_DESTINATION_NAME"))?;
    let lock_path = parent.join(format!(".{name}.stage1-lock"));
    let mut lock = OpenOptions::new().write(true).create_new(true).open(&lock_path)
        .map_err(|error| if error.kind() == std::io::ErrorKind::AlreadyExists {
            conflict("STAGE1_FILE_BUSY: another writer or an interrupted write holds this file; retain input and choose a fresh copy if needed")
        } else { io_failure(error) })?;
    let _guard = CooperativeLock(lock_path);
    serde_json::to_writer(&mut lock, &job.ticket).map_err(io_failure)?;
    lock.sync_all().map_err(io_failure)?;
    if let Some(prior) = check_destination(path, job.expected_disk_hash.as_deref())? {
        retain_prior(parent, &prior)?;
    }
    // MBSE-Craft integration: keep app-owned admission, frozen snapshots,
    // cooperative locking and history while delegating the durable commit to
    // the actual pinned ArtCraft atomic engine. New copies never overwrite.
    check_destination(path, job.expected_disk_hash.as_deref())?;
    let options = craft_atomic::WriteOptions::default()
        .with_new_file_permissions(craft_atomic::NewFilePermissions::Private);
    if job.expected_disk_hash.is_some() {
        options.atomic_write(path, &job.bytes).map_err(io_failure)?;
    } else {
        options.write_new(path, &job.bytes).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                conflict("STAGE1_DISK_CONFLICT")
            } else {
                io_failure(error)
            }
        })?;
    }
    sync_parent(parent)?;
    let committed = read_bounded(path).map_err(io_failure)?;
    if committed != job.bytes {
        return Err(conflict("STAGE1_POST_WRITE_CONFLICT"));
    }
    Ok(WriteReceipt {
        destination: path.clone(),
        envelope_hash: hash(&committed),
        snapshot_hash: job.ticket.snapshot_hash.clone(),
        written_bytes: committed.len() as u64,
        _verified_write: (),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::{Command, RawKey, RecordId};
    use std::collections::BTreeMap;

    fn area() -> tempfile::TempDir {
        // Integrator supplies an admitted TMPDIR; never use the root disk.
        let root =
            std::env::var_os("TMPDIR").expect("Stage1 tests require explicit workspace TMPDIR");
        assert!(Path::new(&root).starts_with("/workspace") || cfg!(windows));
        tempfile::Builder::new()
            .prefix("stage1-io-test-")
            .tempdir_in(root)
            .unwrap()
    }
    fn rename(session: &mut Session, name: &str) {
        session
            .editor
            .command(
                &session.editor.freshness(),
                Command::EditText {
                    key: RawKey {
                        record: RecordId(1),
                        field: "name".into(),
                        item: None,
                    },
                    text: name.into(),
                },
            )
            .unwrap();
    }
    fn save(session: &mut Session, path: &Path) {
        let job = session.begin_save(path).unwrap();
        let result = execute_write(job);
        assert!(result.succeeded(), "{:?}", result.error());
        assert!(session.complete(result));
    }

    #[test]
    fn frozen_older_save_acknowledges_actual_bytes_but_live_remains_dirty() {
        let area = area();
        let path = area.path().join("model.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        rename(&mut session, "First accepted name");
        let job = session.begin_save(&path).unwrap();
        let frozen = job.snapshot.clone();
        rename(&mut session, "Live later name");
        let result = execute_write(job);
        let duplicate = result.clone();
        assert!(session.complete(result));
        assert!(!session.complete(duplicate));
        assert_eq!(read_snapshot(&path).unwrap(), frozen);
        assert!(session.is_dirty());
        assert_eq!(session.status(), SaveStatus::Unsaved);
        save(&mut session, &path);
        assert_eq!(session.status(), SaveStatus::Saved);
        assert_eq!(read_snapshot(&path).unwrap(), session.editor.snapshot);
        assert!(area.path().join(".kernspan-stage1-history").is_dir());
    }

    #[test]
    fn full_snapshot_roundtrip_keeps_opaque_fields_invalid_raw_and_creation_draft() {
        let area = area();
        let path = area.path().join("model.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        rename(&mut session, "Retained draft");
        session
            .editor
            .snapshot
            .records
            .get_mut(&RecordId(1))
            .unwrap()
            .fields
            .insert(
                "opaque_extension".into(),
                serde_json::json!({"original":"bytes", "nested":[1, false]}),
            );
        session
            .editor
            .snapshot
            .drafts
            .push(crate::stage1::RawDraft {
                key: RawKey {
                    record: RecordId(9999),
                    field: "criterion".into(),
                    item: None,
                },
                text: "9e".into(),
                version: 7,
                validity: crate::stage1::Validity::Incomplete,
                unit_context: Some("percent".into()),
                owner_generation: None,
            });
        session.editor.snapshot.raw_version = 7;
        session.editor.snapshot.next_id = 10_000;
        session.editor.snapshot.creation_drafts.insert(
            "mission-draft".into(),
            crate::stage1::CreationDraft {
                owner: RecordId(1),
                kind: crate::stage1::RecordKind::Mission,
                fields: BTreeMap::from([("name".into(), "unfinished mission".into())]),
            },
        );
        let expected = session.editor.snapshot.clone();
        save(&mut session, &path);
        let reopened = Session::open(&area.path().join("state"), &path).unwrap();
        assert_eq!(reopened.editor.snapshot, expected);
        assert!(!reopened.editor.can_undo());
        assert!(!reopened.editor.can_redo());
        assert!(!reopened.is_dirty());
        assert_ne!(reopened.session_id(), session.session_id());
        assert_ne!(reopened.instance_id(), session.instance_id());
        assert!(session.reservation.exists());
    }

    #[test]
    fn foreign_and_duplicate_ack_never_clear_active_request() {
        let area = area();
        let mut first = Session::new(&area.path().join("state")).unwrap();
        let mut second = Session::new(&area.path().join("state")).unwrap();
        let first_job = first
            .begin_save(&area.path().join("first.kernspan"))
            .unwrap();
        let second_job = second
            .begin_save(&area.path().join("second.kernspan"))
            .unwrap();
        let second_result = execute_write(second_job);
        assert!(!first.complete(second_result.clone()));
        assert!(first.pending());
        let first_result = execute_write(first_job);
        assert!(first.complete(first_result.clone()));
        let next_job = first
            .begin_save(&area.path().join("first.kernspan"))
            .unwrap();
        assert!(!first.complete(first_result));
        assert!(first.pending());
        assert!(first.complete(execute_write(next_job)));
        assert!(second.complete(second_result));
    }

    #[test]
    fn external_conflict_and_unknown_target_preserve_live_input_and_source() {
        let area = area();
        let path = area.path().join("model.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        save(&mut session, &path);
        rename(&mut session, "Unsaved live");
        let job = session.begin_save(&path).unwrap();
        let outsider = b"an external edit, never valid Stage1";
        fs::write(&path, outsider).unwrap();
        let result = execute_write(job);
        assert!(result.error().unwrap().conflict);
        assert!(session.complete(result));
        assert!(matches!(session.status(), SaveStatus::Conflict(_)));
        assert_eq!(fs::read(&path).unwrap(), outsider);
        assert!(session.is_dirty());
        assert_eq!(
            session.editor.snapshot.records[&RecordId(1)].name,
            "Unsaved live"
        );
        let sysml = area.path().join("source.sysml");
        fs::write(&sysml, b"package Untouched;").unwrap();
        assert!(session.begin_save(&sysml).is_err());
        assert_eq!(fs::read(&sysml).unwrap(), b"package Untouched;");
    }

    #[test]
    fn recovery_is_actual_independent_dirty_copy_with_protected_original() {
        let area = area();
        let state = area.path().join("state");
        let path = area.path().join("source.kernspan");
        let mut session = Session::new(&state).unwrap();
        save(&mut session, &path);
        let original = fs::read(&path).unwrap();
        rename(&mut session, "Recovered live input");
        let expected = session.editor.snapshot.clone();
        let job = session.begin_recovery().unwrap();
        let destination = job.ticket.destination.clone();
        assert!(session.complete(execute_write(job)));
        assert!(session.recovery_current());
        assert!(session.is_dirty());
        assert_eq!(session.status(), SaveStatus::Unsaved);
        assert!(Session::open(&state, &destination).is_err());
        let mut restored = Session::restore(&state, &destination).unwrap();
        assert!(restored.path.is_none());
        assert!(restored.is_dirty());
        assert_ne!(restored.editor.snapshot.document_id, expected.document_id);
        assert_eq!(restored.editor.snapshot.records, expected.records);
        assert_eq!(restored.editor.snapshot.drafts, expected.drafts);
        assert_eq!(
            restored.editor.snapshot.creation_drafts,
            expected.creation_drafts
        );
        assert!(!restored.editor.can_undo());
        assert!(restored.begin_save(&path).is_err());
        assert!(restored.begin_save(&destination).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        save(&mut restored, &area.path().join("copy.kernspan"));
        assert_eq!(restored.status(), SaveStatus::Saved);
    }

    #[test]
    fn original_source_path_hash_pairs_survive_recovery_copy_reopen_generations() {
        let area = area();
        let state = area.path().join("state");
        let original_path = area.path().join("original.kernspan");
        let mut original = Session::new(&state).unwrap();
        rename(&mut original, "Original accepted state");
        save(&mut original, &original_path);
        let original_bytes = fs::read(&original_path).unwrap();
        let original_hash = hash(&original_bytes);
        // The native destination identity is OS-canonical. On Windows this
        // includes the verbatim path prefix; preserve it in stored provenance.
        let canonical_original = fs::canonicalize(&original_path).unwrap();
        assert_eq!(original.path.as_ref(), Some(&canonical_original));
        rename(&mut original, "First recovered state");
        let result = execute_write(original.begin_recovery().unwrap());
        let recovery_one = result.ticket.destination.clone();
        assert!(original.complete(result));
        let first: Value = serde_json::from_slice(&fs::read(&recovery_one).unwrap()).unwrap();
        assert_eq!(
            first["provenance"]["source_path"],
            serde_json::to_value(&canonical_original).unwrap()
        );
        assert_eq!(
            first["provenance"]["source_hash"],
            Value::String(original_hash.clone())
        );

        let mut copy = Session::restore(&state, &recovery_one).unwrap();
        let copy_path = area.path().join("independent-copy.kernspan");
        save(&mut copy, &copy_path);
        // Inspect actual persisted native bytes, not the in-memory provenance.
        fn has_pair(path: &Path, source: &Path, expected_hash: &str) -> bool {
            let value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            let canonical_source = fs::canonicalize(source).unwrap();
            value["provenance"]["protected_sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| {
                    entry["path"] == serde_json::to_value(&canonical_source).unwrap()
                        && entry["hash"].as_str() == Some(expected_hash)
                })
        }
        assert!(has_pair(&copy_path, &original_path, &original_hash));
        let mut reopened = Session::open(&state, &copy_path).unwrap();
        rename(&mut reopened, "Second recovered state");
        assert!(reopened.begin_save(&original_path).is_err());
        assert!(reopened.begin_save(&canonical_original).is_err());
        let result = execute_write(reopened.begin_recovery().unwrap());
        let recovery_two = result.ticket.destination.clone();
        assert!(reopened.complete(result));
        assert!(has_pair(&recovery_two, &original_path, &original_hash));

        let mut restored_twice = Session::restore(&state, &recovery_two).unwrap();
        assert!(restored_twice.begin_save(&original_path).is_err());
        assert!(restored_twice.begin_save(&canonical_original).is_err());
        assert!(restored_twice.begin_save(&copy_path).is_err());
        let second_copy = area.path().join("second-independent-copy.kernspan");
        save(&mut restored_twice, &second_copy);
        assert!(has_pair(&second_copy, &original_path, &original_hash));
        assert!(has_pair(
            &second_copy,
            &copy_path,
            &hash(&fs::read(&copy_path).unwrap())
        ));
        assert_eq!(fs::read(&original_path).unwrap(), original_bytes);
    }

    #[test]
    fn failed_or_truncated_recovery_never_acknowledges_live_input() {
        let area = area();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let job = session.begin_recovery().unwrap();
        let result = execute_write(job);
        let path = result.ticket.destination.clone();
        fs::write(&path, b"truncated").unwrap();
        assert!(session.complete(result));
        assert!(matches!(session.status(), SaveStatus::Conflict(_)));
        assert!(!session.recovery_current());
        assert!(session.last_recovery().is_none());
        assert!(session.is_dirty());
        // The unacknowledged file cannot be silently overwritten on retry.
        let result = execute_write(session.begin_recovery().unwrap());
        assert!(!result.succeeded());
        assert!(session.complete(result));
        assert!(!session.recovery_current());
    }

    #[test]
    fn recovery_current_rechecks_acknowledged_bytes_before_lifecycle_permission() {
        let area = area();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let result = execute_write(session.begin_recovery().unwrap());
        assert!(session.complete(result));
        assert!(session.recovery_current());
        let path = session.last_recovery().unwrap().to_owned();
        fs::write(path, b"later truncation").unwrap();
        assert!(!session.recovery_current());
        assert!(session.last_recovery().is_some());
        assert!(session.is_dirty());
    }

    #[test]
    fn older_recovery_does_not_acknowledge_later_input() {
        let area = area();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let job = session.begin_recovery().unwrap();
        rename(&mut session, "After frozen recovery");
        assert!(session.complete(execute_write(job)));
        assert!(!session.recovery_current());
        let job = session.begin_recovery().unwrap();
        assert!(session.complete(execute_write(job)));
        assert!(session.recovery_current());
        assert!(session.is_dirty());
    }

    #[test]
    fn autosave_known_destination_uses_actual_save_protocol_and_toggle_has_no_io() {
        let area = area();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let before: Vec<_> = fs::read_dir(&session.reservation).unwrap().collect();
        session.set_autosave(true);
        session.set_autosave(true);
        assert!(session.begin_autosave().unwrap().is_none());
        assert_eq!(
            fs::read_dir(&session.reservation).unwrap().count(),
            before.len()
        );
        assert_eq!(session.status(), SaveStatus::Unsaved);
        let path = area.path().join("model.kernspan");
        save(&mut session, &path);
        assert!(session.begin_autosave().unwrap().is_none());
        rename(&mut session, "Autosaved live");
        let job = session.begin_autosave().unwrap().unwrap();
        assert!(session.begin_autosave().unwrap().is_none());
        session.set_autosave(false);
        assert!(session.pending());
        assert!(session.complete(execute_write(job)));
        assert_eq!(session.status(), SaveStatus::Saved);
        assert_eq!(read_snapshot(&path).unwrap(), session.editor.snapshot);
    }

    #[test]
    fn reader_rejects_duplicates_unknown_profiles_and_oversize_before_open() {
        let area = area();
        let path = area.path().join("model.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        save(&mut session, &path);
        let bytes = fs::read(&path).unwrap();
        let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
            "\"profile\":",
            "\"profile\":\"kernspan.stage1/1\",\"profile\":",
            1,
        );
        fs::write(&path, duplicate).unwrap();
        assert!(read_snapshot(&path).unwrap_err().contains("duplicate"));
        let nested_duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
            "\"name\":",
            "\"name\":\"first\",\"name\":",
            1,
        );
        fs::write(&path, nested_duplicate).unwrap();
        assert!(read_snapshot(&path).unwrap_err().contains("duplicate"));
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["schema_version"] = u32::MAX.into();
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_snapshot(&path)
            .unwrap_err()
            .contains("UNSUPPORTED_PROFILE_OR_VERSION"));
        let mut unknown: Value = serde_json::from_slice(&bytes).unwrap();
        unknown["snapshot"]["future_data"] = true.into();
        fs::write(&path, serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(read_snapshot(&path).unwrap_err().contains("UNKNOWN_FIELD"));
        let mut alias: Value = serde_json::from_slice(&bytes).unwrap();
        let original_record = alias["snapshot"]["records"]["1"].clone();
        alias["snapshot"]["records"]["01"] = original_record;
        fs::write(&path, serde_json::to_vec(&alias).unwrap()).unwrap();
        assert!(read_snapshot(&path).unwrap_err().contains("NONCANONICAL"));
        let oversized = File::create(&path).unwrap();
        oversized.set_len((MAX_BYTES + 1) as u64).unwrap();
        assert!(read_snapshot(&path).unwrap_err().contains("LIMIT"));
    }

    #[test]
    fn cooperative_lock_failure_is_not_saved_and_retry_preserves_input() {
        let area = area();
        let path = area.path().join("model.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        rename(&mut session, "Still here");
        let job = session.begin_save(&path).unwrap();
        let lock = area.path().join(".model.kernspan.stage1-lock");
        fs::write(&lock, b"another cooperative writer").unwrap();
        let result = execute_write(job);
        assert!(!result.succeeded());
        assert!(session.complete(result));
        assert!(session.is_dirty());
        assert!(!path.exists());
        assert_eq!(fs::read(&lock).unwrap(), b"another cooperative writer");
        fs::remove_file(&lock).unwrap();
        save(&mut session, &path);
        assert_eq!(session.status(), SaveStatus::Saved);
    }
    #[test]
    fn reader_refuses_nonzero_numeric_underflow_without_changing_source() {
        let area = area();
        let path = area.path().join("underflow.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        save(&mut session, &path);
        let original = fs::read_to_string(&path).unwrap();
        let mut envelope: Value = serde_json::from_str(&original).unwrap();
        envelope["snapshot"]["records"]["1"]["fields"]["opaque_numeric_probe"] =
            "UNDERFLOW_TEST_TOKEN".into();
        let changed = serde_json::to_string(&envelope).unwrap().replacen(
            "\"UNDERFLOW_TEST_TOKEN\"",
            "1e-9999",
            1,
        );
        assert_ne!(changed, original);
        fs::write(&path, &changed).unwrap();
        assert!(read_snapshot(&path)
            .unwrap_err()
            .contains("UNSUPPORTED_NUMERIC_RANGE"));
        assert_eq!(fs::read_to_string(&path).unwrap(), changed);
        assert!(reject_numeric_underflow(br#"{"zero":0e-9999}"#).is_ok());
        assert!(reject_numeric_underflow(br#"{"text":"1e-9999"}"#).is_ok());
    }

    #[test]
    fn retained_lock_injection_preserves_existing_file_and_allows_fresh_copy() {
        let area = area();
        let path = area.path().join("original.kernspan");
        let alternate = area.path().join("fresh-copy.kernspan");
        let mut session = Session::new(&area.path().join("state")).unwrap();
        save(&mut session, &path);
        let original = fs::read(&path).unwrap();
        let lock_path = area.path().join(".original.kernspan.stage1-lock");
        fs::write(&lock_path, b"injected retained lock; no automatic takeover").unwrap();
        rename(&mut session, "Retained input after interruption");
        let result = execute_write(session.begin_save(&path).unwrap());
        assert!(!result.succeeded());
        assert!(result.error().unwrap().message.contains("interrupted"));
        assert!(session.complete(result));
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(Session::open(&area.path().join("reader-state"), &path).is_ok());
        save(&mut session, &alternate);
        assert_eq!(
            read_snapshot(&alternate).unwrap().records[&crate::stage1::RecordId(1)].name,
            "Retained input after interruption"
        );
        assert_eq!(
            fs::read(&lock_path).unwrap(),
            b"injected retained lock; no automatic takeover"
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}
