//! A retained, explicitly bound name draft; no native-model access.
use eframe::egui;
use mbse_exchange::{
    source_edit::{EditReceipt, NameProposal},
    v2::{ElementKey, ProjectionSet, MAX_VALUE_BYTES},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};

struct Draft {
    source_path: PathBuf,
    source: Arc<ProjectionSet>,
    key: ElementKey,
    raw: String,
    prepared: Option<NameProposal>,
}
#[derive(Debug)]
enum Answer {
    Prepared(NameProposal),
    Created(PathBuf, EditReceipt),
}
#[derive(Default)]
pub struct NameEditor {
    draft: Option<Draft>,
    generation: u64,
    pending: Option<Receiver<(u64, Result<Answer, String>)>>,
    writing: bool,
    destination: String,
    receipt: Option<(PathBuf, EditReceipt)>,
    feedback: String,
    error: bool,
    bounds: BTreeMap<String, [f32; 4]>,
}
impl NameEditor {
    pub fn blocks_source_change(&self) -> bool {
        self.draft.is_some() || self.pending.is_some()
    }
    #[cfg(test)]
    pub(crate) fn retained_binding(&self) -> Option<(ElementKey, String)> {
        self.draft.as_ref().map(|d| (d.key.clone(), d.raw.clone()))
    }
    pub fn dirty(&self) -> bool {
        self.draft.is_some()
    }
    pub fn write_pending(&self) -> bool {
        self.writing && self.pending.is_some()
    }
    pub fn request_close(&mut self) -> bool {
        if !self.guards_close() {
            return true;
        }
        self.feedback = self.close_message().into();
        self.error = false;
        false
    }
    fn discard(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.draft = None;
        self.generation = self.generation.saturating_add(1);
        self.feedback="Retained name proposal explicitly discarded. Frozen source and native model unchanged.".into();
        self.error = false;
        true
    }
    fn open_target(&self) -> Option<(PathBuf, String)> {
        if self.blocks_source_change() {
            return None;
        }
        self.receipt
            .as_ref()
            .map(|(path, receipt)| (path.clone(), receipt.output_source_set_digest.clone()))
    }
    pub fn guards_close(&self) -> bool {
        self.blocks_source_change()
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn close_message(&self) -> &'static str {
        if self.writing {
            "Close canceled while the new source copy is being written. Wait for its acknowledgment; the name proposal remains retained."
        } else {
            "Close canceled: an unsaved name proposal is retained. Create its new copy or explicitly discard the proposal before closing."
        }
    }
    pub fn begin(
        &mut self,
        path: PathBuf,
        source: Arc<ProjectionSet>,
        key: ElementKey,
        raw: String,
    ) {
        if self.blocks_source_change() {
            return;
        }
        self.generation = self.generation.saturating_add(1);
        self.draft = Some(Draft {
            source_path: path,
            source,
            key,
            raw,
            prepared: None,
        });
        self.feedback="Raw name draft retained separately from the frozen source and native model. Prepare it to inspect exact byte preconditions.".into();
        self.error = false;
    }
    fn bound(&mut self, key: &str, response: &egui::Response) {
        let r = response.interact_rect;
        self.bounds
            .insert(key.into(), [r.min.x, r.min.y, r.max.x, r.max.y]);
    }
    pub fn take_bounds(&mut self) -> BTreeMap<String, [f32; 4]> {
        std::mem::take(&mut self.bounds)
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        let answer=self.pending.as_ref().and_then(|rx|match rx.try_recv(){
            Ok(value)=>Some(value),Err(mpsc::TryRecvError::Empty)=>None,
            Err(mpsc::TryRecvError::Disconnected)=>Some((self.generation,Err("Source-edit worker disconnected. Draft and destination retained; inspect any possible output before retrying.".into())))
        });
        let Some((generation, answer)) = answer else {
            return;
        };
        self.pending = None;
        self.writing = false;
        match answer {
            Ok(Answer::Prepared(proposal)) => {
                if generation == self.generation
                    && self.draft.as_ref().is_some_and(|d| {
                        d.key == proposal.element
                            && d.raw == proposal.proposed_value
                            && d.source.source_set_digest == proposal.source_set_digest
                    })
                {
                    self.draft.as_mut().unwrap().prepared = Some(proposal);
                    self.feedback =
                        "Exact name proposal prepared. No output file has been written.".into();
                    self.error = false;
                } else {
                    self.feedback =
                        "Stale preparation acknowledgment refused; current raw draft retained."
                            .into();
                    self.error = true;
                }
            }
            Ok(Answer::Created(path, receipt)) => {
                let current = generation == self.generation
                    && self.draft.as_ref().is_some_and(|d| {
                        d.source.source_set_digest == receipt.original_source_set_digest
                            && d.source.primary_sha256 == receipt.original_primary_sha256
                            && d.prepared.as_ref().is_some_and(|p| {
                                p.element == d.key
                                    && p.proposed_value == d.raw
                                    && p.source_set_digest == d.source.source_set_digest
                            })
                    });
                self.receipt = Some((path, receipt));
                if current {
                    self.draft = None;
                    self.feedback="New source copy acknowledged. Original source and active preview are unchanged. Inspect the receipt, then explicitly open the new copy.".into();
                    self.error = false;
                } else {
                    self.feedback="Output was created, but its acknowledgment does not match the current draft generation. Preserve the output and current draft; inspect the receipt.".into();
                    self.error = true;
                }
            }
            Err(error) => {
                self.feedback = if error.contains("SOURCE_EDIT_OUTPUT_CREATED_") {
                    format!("OUTPUT EXISTS: preserve the created output for inspection. Raw proposal and destination are retained. {error}")
                } else {
                    format!("Source edit was not acknowledged. Proposal and destination retained. Inspect any existing output before retrying. {error}")
                };
                self.error = true;
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(PathBuf, String)> {
        let mut prepare = false;
        let mut create = false;
        let mut discard = false;
        let mut open = None;
        if self.draft.is_some() || self.receipt.is_some() || !self.feedback.is_empty() {
            ui.heading("Source name proposal");
            ui.colored_label(
                if self.error {
                    egui::Color32::DARK_RED
                } else {
                    egui::Color32::from_rgb(25, 65, 70)
                },
                &self.feedback,
            );
        }
        if let Some(draft) = &mut self.draft {
            wrapped(ui,format!("Bound source: {}\nExact scoped key: {:?}\nFrozen primary SHA256: {}\nSource-set SHA256: {}\nDraft generation: {}",draft.source_path.display(),draft.key,draft.source.primary_sha256,draft.source.source_set_digest,self.generation));
            ui.label("Only an existing unqualified name on an owned identified Capella 7 Mission, Capability or SystemComponent can change. Direct UTF-8 XML 1.0 only; no archive edit, global semantic rename, native admission or execution. Name-based consumers and external-tool acceptance remain unverified.");
            let response = ui.add_enabled(
                self.pending.is_none(),
                egui::TextEdit::multiline(&mut draft.raw)
                    .desired_rows(2)
                    .desired_width(ui.available_width())
                    .char_limit(MAX_VALUE_BYTES),
            );
            let changed = response.changed();
            let r = response.interact_rect;
            self.bounds.insert(
                "semantic:name-draft".into(),
                [r.min.x, r.min.y, r.max.x, r.max.y],
            );
            if changed {
                draft.prepared = None;
                self.generation = self.generation.saturating_add(1);
            }
            ui.label(format!("Raw proposal: {} UTF-8 bytes; preparation limit {} bytes. Invalid/oversized proposals remain retained on refusal.",draft.raw.len(),MAX_VALUE_BYTES));
            let response = ui.add_enabled(
                self.pending.is_none(),
                egui::Button::new("Prepare exact name proposal"),
            );
            let r = response.interact_rect;
            self.bounds.insert(
                "semantic:prepare-name".into(),
                [r.min.x, r.min.y, r.max.x, r.max.y],
            );
            prepare = response.clicked();
            if let Some(proposal) = &draft.prepared {
                wrapped(ui,format!("Prepared contract: {}\nBound source-set SHA256: {}\nField: unqualified name\nNamespace: {}\nClassifier: {}\nOriginal byte span: {}..{}",proposal.profile,proposal.source_set_digest,proposal.source_namespace,proposal.source_classifier,proposal.value_range.start,proposal.value_range.end));
                wrapped(ui, format!("Exact scoped key: {:?}", proposal.element));
                wrapped(
                    ui,
                    format!("Original decoded value: {:?}", proposal.original_value),
                );
                wrapped(
                    ui,
                    format!(
                        "Original encoded value: {:?}",
                        proposal.original_encoded_value
                    ),
                );
                wrapped(
                    ui,
                    format!("Proposed decoded value: {:?}", proposal.proposed_value),
                );
                ui.label("The worker rereads and rebinds the complete source, refuses stale bytes, patches only this value, reparses the bounded projection and creates a private NEW destination. IDs and reference spellings remain unchanged; engineering validity is unverified.");
            }
            ui.label("New destination; existing leaf entries are refused");
            let response = ui.add_enabled(
                self.pending.is_none(),
                egui::TextEdit::singleline(&mut self.destination)
                    .desired_width(ui.available_width())
                    .hint_text("/workspace/…/NEW-file.capella"),
            );
            let r = response.interact_rect;
            self.bounds.insert(
                "semantic:name-destination".into(),
                [r.min.x, r.min.y, r.max.x, r.max.y],
            );
            let response = ui.add_enabled(
                self.pending.is_none()
                    && draft.prepared.is_some()
                    && !self.destination.trim().is_empty(),
                egui::Button::new("Create new copy with prepared name"),
            );
            let r = response.interact_rect;
            self.bounds.insert(
                "semantic:create-name-copy".into(),
                [r.min.x, r.min.y, r.max.x, r.max.y],
            );
            create = response.clicked();
            let response = ui.add_enabled(
                self.pending.is_none(),
                egui::Button::new("Discard retained name proposal"),
            );
            let r = response.interact_rect;
            self.bounds.insert(
                "semantic:discard-name-proposal".into(),
                [r.min.x, r.min.y, r.max.x, r.max.y],
            );
            discard = response.clicked();
        }
        if discard {
            self.discard();
        }
        if prepare {
            self.prepare();
        } else if create {
            self.create();
        }
        if let Some((path, receipt)) = &self.receipt {
            ui.heading("New-copy edit receipt");
            wrapped(ui,format!("New copy: {}\nContract: {}\nOriginal SHA256: {}\nOutput SHA256: {}\nOriginal source-set SHA256: {}\nOutput source-set SHA256: {}\nChanged original byte span: {:?}\nEncoded bytes: {} -> {}\nOutside patch bytes unchanged: {}\nXML reparsed: {}\nPersisted IDs/reference spellings unchanged: {}\nEngineering validated: {}",path.display(),receipt.profile,receipt.original_primary_sha256,receipt.output_primary_sha256,receipt.original_source_set_digest,receipt.output_source_set_digest,receipt.changed_value_range,receipt.original_encoded_bytes,receipt.replacement_encoded_bytes,receipt.outside_patch_bytes_unchanged,receipt.xml_reparsed,receipt.persisted_ids_and_reference_spellings_unchanged,receipt.engineering_validated));
            for limitation in &receipt.limitations {
                wrapped(ui, limitation);
            }
            let response = ui.add_enabled(
                !self.blocks_source_change(),
                egui::Button::new("Open acknowledged new source copy"),
            );
            if response.clicked() {
                open = self.open_target();
            }
            self.bound("semantic:open-name-copy", &response);
        }
        open
    }
    fn prepare(&mut self) {
        let Some(draft) = &self.draft else {
            return;
        };
        if self.pending.is_some() {
            return;
        }
        let source = draft.source.clone();
        let key = draft.key.clone();
        let raw = draft.raw.clone();
        let generation = self.generation;
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.feedback = "Preparing exact frozen-source name preconditions…".into();
        self.error = false;
        std::thread::spawn(move || {
            let result = NameProposal::prepare(&source, &key, raw).map(Answer::Prepared);
            let _ = tx.send((generation, result));
        });
    }
    fn create(&mut self) {
        let path = PathBuf::from(self.destination.trim());
        if !valid_destination(&path) {
            self.feedback="Choose an absolute NEW destination with an existing parent on /workspace; parent traversal is refused.".into();
            self.error = true;
            return;
        }
        let Some(draft) = &self.draft else {
            return;
        };
        let Some(proposal) = draft.prepared.clone() else {
            return;
        };
        if self.pending.is_some() {
            return;
        }
        let source = draft.source_path.clone();
        let generation = self.generation;
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.writing = true;
        self.feedback="Creating a new source copy from exact prepared preconditions; waiting for acknowledgment…".into();
        self.error = false;
        std::thread::spawn(move || {
            let result = proposal
                .create_new_copy(&source, &path)
                .map(|receipt| Answer::Created(path, receipt));
            let _ = tx.send((generation, result));
        });
    }
}
fn valid_destination(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        && path
            .parent()
            .and_then(|p| std::fs::canonicalize(p).ok())
            .is_some_and(|p| p.starts_with("/workspace"))
}
fn wrapped(ui: &mut egui::Ui, text: impl Into<String>) {
    let mut text = text.into();
    if text.len() > 16 * 1024 {
        let mut end = 16 * 1024;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str(
            "\n[Display clipped; bound proposal and exact frozen source remain retained.]",
        );
    }
    ui.add(egui::Label::new(text).wrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use mbse_exchange::v2::{self, SemanticKind};
    fn source(name: &str) -> Arc<ProjectionSet> {
        let xml = format!(
            r#"<root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ctx="http://www.polarsys.org/capella/core/ctx/7.0.0"><ownedMissions xsi:type="ctx:Mission" id="mission-1" name="{name}"/><ownedCapabilities xsi:type="ctx:Capability" id="capability-2" name="Capability"/></root>"#
        );
        Arc::new(v2::inspect_primary_bytes("synthetic.capella", xml.into_bytes()).unwrap())
    }
    fn editor() -> NameEditor {
        let set = source("Original");
        let node = set
            .nodes
            .iter()
            .find(|n| n.kind == SemanticKind::CapellaMission)
            .unwrap();
        let mut editor = NameEditor::default();
        editor.begin(
            PathBuf::from("/workspace/synthetic.capella"),
            set.clone(),
            node.key.clone(),
            "New & retained".into(),
        );
        editor.destination = "/workspace/new-copy.capella".into();
        editor
    }
    fn proposal(editor: &NameEditor) -> NameProposal {
        let draft = editor.draft.as_ref().unwrap();
        NameProposal::prepare(&draft.source, &draft.key, draft.raw.clone()).unwrap()
    }
    fn receipt(proposal: &NameProposal) -> EditReceipt {
        EditReceipt {
            profile: "mbse-craft.external.xml-edit/1",
            original_source_set_digest: proposal.source_set_digest.clone(),
            output_source_set_digest: "expected-output-set-digest".into(),
            original_primary_sha256: proposal.primary_sha256.clone(),
            output_primary_sha256: "expected-output-primary".into(),
            changed_value_range: proposal.value_range.clone(),
            original_encoded_bytes: proposal.value_range.len(),
            replacement_encoded_bytes: 20,
            outside_patch_bytes_unchanged: true,
            xml_reparsed: true,
            persisted_ids_and_reference_spellings_unchanged: true,
            engineering_validated: false,
            limitations: vec![
                "Injected acknowledgment tests controller policy; no actual write is claimed"
                    .into(),
            ],
        }
    }
    fn deliver(
        editor: &mut NameEditor,
        generation: u64,
        answer: Result<Answer, String>,
        writing: bool,
    ) {
        let (tx, rx) = mpsc::channel();
        editor.pending = Some(rx);
        editor.writing = writing;
        tx.send((generation, answer)).unwrap();
        editor.poll(&egui::Context::default());
    }
    #[test]
    fn delayed_preparation_cannot_replace_changed_raw_or_source() {
        let mut e = editor();
        let old = proposal(&e);
        let generation = e.generation;
        e.draft.as_mut().unwrap().raw = "Newer unfinished text".into();
        e.generation += 1;
        deliver(&mut e, generation, Ok(Answer::Prepared(old)), false);
        assert_eq!(e.draft.as_ref().unwrap().raw, "Newer unfinished text");
        assert!(e.draft.as_ref().unwrap().prepared.is_none());
        assert!(e.error);
        let old = proposal(&e);
        let generation = e.generation;
        let changed = source("Externally changed");
        let draft = e.draft.as_mut().unwrap();
        draft.key = changed
            .nodes
            .iter()
            .find(|n| n.kind == SemanticKind::CapellaMission)
            .unwrap()
            .key
            .clone();
        draft.source = changed;
        deliver(&mut e, generation, Ok(Answer::Prepared(old)), false);
        assert!(e.draft.as_ref().unwrap().prepared.is_none());
        assert!(e.error);
    }
    #[test]
    fn existing_draft_binding_survives_other_node_begin_and_close() {
        let mut e = editor();
        let key = e.draft.as_ref().unwrap().key.clone();
        let set = source("Other source");
        let other = set
            .nodes
            .iter()
            .find(|n| n.kind == SemanticKind::CapellaCapability)
            .unwrap()
            .key
            .clone();
        e.begin(
            PathBuf::from("/workspace/other.capella"),
            set,
            other,
            "Other".into(),
        );
        assert_eq!(e.draft.as_ref().unwrap().key, key);
        assert_eq!(e.draft.as_ref().unwrap().raw, "New & retained");
        assert!(!e.request_close());
        assert!(!e.write_pending());
        assert!(e.dirty());
        assert!(e.discard());
        assert!(e.request_close());
    }
    #[test]
    fn pending_write_refuses_discard_close_and_source_replacement() {
        let mut e = editor();
        let (tx, rx) = mpsc::channel();
        e.pending = Some(rx);
        e.writing = true;
        assert!(!e.discard());
        assert!(!e.request_close());
        assert!(e.write_pending());
        let set = source("Replacement");
        let key = set
            .nodes
            .iter()
            .find(|n| n.kind == SemanticKind::CapellaMission)
            .unwrap()
            .key
            .clone();
        e.begin(
            PathBuf::from("/workspace/replacement.capella"),
            set,
            key,
            "Replacement".into(),
        );
        assert_eq!(e.draft.as_ref().unwrap().raw, "New & retained");
        tx.send((e.generation, Err("SOURCE_EDIT_STALE_SOURCE".into())))
            .unwrap();
        e.poll(&egui::Context::default());
        assert!(!e.write_pending());
        assert!(e.dirty());
        assert_eq!(e.destination, "/workspace/new-copy.capella");
        assert!(!e.request_close());
    }
    #[test]
    fn matching_copy_ack_clears_only_its_draft_and_exposes_exact_digest_open() {
        let mut e = editor();
        let p = proposal(&e);
        let r = receipt(&p);
        e.draft.as_mut().unwrap().prepared = Some(p);
        assert!(e.open_target().is_none());
        let generation = e.generation;
        deliver(
            &mut e,
            generation,
            Ok(Answer::Created(
                PathBuf::from("/workspace/acknowledged.capella"),
                r,
            )),
            true,
        );
        assert!(e.draft.is_none());
        assert!(!e.error);
        assert!(e.request_close());
        assert_eq!(
            e.open_target(),
            Some((
                PathBuf::from("/workspace/acknowledged.capella"),
                "expected-output-set-digest".into()
            ))
        );
    }
    #[test]
    fn stale_copy_ack_and_created_output_error_preserve_draft_and_destination() {
        let mut e = editor();
        let p = proposal(&e);
        let r = receipt(&p);
        let generation = e.generation;
        e.draft.as_mut().unwrap().prepared = Some(p);
        e.generation += 1;
        deliver(
            &mut e,
            generation,
            Ok(Answer::Created(
                PathBuf::from("/workspace/stale-created.capella"),
                r,
            )),
            true,
        );
        assert!(e.dirty());
        assert!(e.receipt.is_some());
        assert!(e.error);
        assert!(e.open_target().is_none());
        assert_eq!(e.destination, "/workspace/new-copy.capella");
        let generation = e.generation;
        deliver(
            &mut e,
            generation,
            Err("SOURCE_EDIT_OUTPUT_CREATED_READ_FAILED: /workspace/new-copy.capella".into()),
            true,
        );
        assert!(e.dirty());
        assert_eq!(e.draft.as_ref().unwrap().raw, "New & retained");
        assert_eq!(e.destination, "/workspace/new-copy.capella");
        assert!(e.feedback.contains("OUTPUT EXISTS"));
        assert!(e.error);
    }
}
