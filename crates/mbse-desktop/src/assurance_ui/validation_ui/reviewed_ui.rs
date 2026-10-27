//! Explicit native rule admission and bounded literal-fix review; no automatic repair.
use super::*;
use validation::{QuickFixPreview, SuiteReview};
#[derive(Clone)]
struct Review {
    proof: SuiteReview,
    form: serde_json::Value,
}
#[derive(Clone)]
struct Fix {
    proof: QuickFixPreview,
    form: serde_json::Value,
    scope: String,
}
#[derive(Default)]
pub(super) struct State {
    review: Option<Review>,
    fix: Option<Fix>,
    note: String,
}
fn blocked(w: &AssuranceWorkspace) -> bool {
    w.validation_navigation_blocked() || w.unadmitted_form()
}
fn review_current(w: &AssuranceWorkspace, r: &Review) -> bool {
    form(w).as_ref() == Some(&r.form)
}
fn fix_current(w: &AssuranceWorkspace, e: &Editor, f: &Fix) -> bool {
    form(w).as_ref() == Some(&f.form)
        && w.validation.selected == f.scope
        && f.proof.check(&w.board, e).is_ok()
}
pub(super) fn audit(w: &AssuranceWorkspace, e: &Editor) -> serde_json::Value {
    serde_json::json!({"note":w.validation.reviewed.note,"blocked":blocked(w),"suite_review":w.validation.reviewed.review.as_ref().map(|r|serde_json::json!({"proposal":r.proof,"current":review_current(w,r)})),"fix_preview":w.validation.reviewed.fix.as_ref().map(|f|serde_json::json!({"preview":f.proof,"current":fix_current(w,e,f),"all_changes_displayable":f.proof.changes().len()<=128}))})
}
pub(super) fn issue_button(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    e: &Editor,
    audit: &mut BTreeMap<String, [f32; 4]>,
    report: &Report,
    issue: &validation::Issue,
) {
    if !issue.quick_fix {
        return;
    }
    if mark(
        audit,
        format!("validation:Preview fix:{}", issue.id),
        ui.add_enabled(!blocked(w), egui::Button::new("Preview explicit fix")),
    )
    .clicked()
    {
        match report.preview_quick_fix(&w.board, e, &issue.id) {
            Ok(proof) => {
                if let Some(form) = form(w) {
                    w.validation.reviewed.fix = Some(Fix {
                        proof,
                        form,
                        scope: w.validation.selected.clone(),
                    });
                    w.validation.reviewed.note="Fix prepared without changing source. Review every before/after value before Apply.".into();
                }
            }
            Err(error) => w.validation.reviewed.note = error,
        }
    }
}
fn text(value: &Option<String>) -> String {
    value
        .as_ref()
        .map(|s| serde_json::to_string(s).expect("text serializes"))
        .unwrap_or_else(|| "no retained draft".into())
}
pub(super) fn show(
    w: &mut AssuranceWorkspace,
    ui: &mut egui::Ui,
    e: &Editor,
    audit: &mut BTreeMap<String, [f32; 4]>,
) {
    ui.label("Custom rules: exact requirement Identifier/Statement; NonBlank, StartsWith or Equals literals. Optional replacement is explicit. No scripts run when data is opened.");
    if mark(
        audit,
        "validation:Preview suite",
        ui.add_enabled(!blocked(w), egui::Button::new("Preview suite for review")),
    )
    .clicked()
    {
        match w.board.preview_validation_suite() {
            Ok(proof) => {
                if let Some(form) = form(w) {
                    w.validation.reviewed.review = Some(Review { proof, form });
                    w.validation.reviewed.note="Suite prepared. Inspect the exact declared rules and replacements, then explicitly admit.".into();
                }
            }
            Err(error) => w.validation.reviewed.note = error,
        }
    }
    if let Some(review) = w.validation.reviewed.review.clone() {
        let current = review_current(w, &review);
        ui.label(format!(
            "Review suite {}. This declares local checks; it does not approve the model.",
            review.proof.name()
        ));
        egui::ScrollArea::both()
            .id_salt("validation-suite-review")
            .max_height(180.)
            .show(ui, |ui| {
                ui.monospace(
                    serde_json::to_string_pretty(review.proof.suite()).expect("suite serializes"),
                );
            });
        if !current {
            ui.colored_label(
                egui::Color32::DARK_RED,
                "Suite review is stale. Preview again after source/form changes.",
            );
        }
        ui.horizontal(|ui|{
   if mark(audit,"validation:Admit reviewed suite",ui.add_enabled(current&&!blocked(w),egui::Button::new("Admit reviewed suite"))).clicked(){
    match w.board.admit_validation_review(&review.proof){
     Ok(())=>{w.epoch=w.epoch.saturating_add(1);w.validation.selected=review.proof.name().into();w.validation.note.clear();w.validation.reviewed.review=None;w.validation.reviewed.note="Reviewed suite admitted to the independent register. Run explicitly against current source.".into();},
     Err(error)=>w.validation.reviewed.note=error,
    }
   }
   if mark(audit,"validation:Cancel suite review",ui.button("Cancel suite review")).clicked(){w.validation.reviewed.review=None;}
  });
    }
    if let Some(fix) = w.validation.reviewed.fix.clone() {
        let current = fix_current(w, e, &fix);
        let count = fix.proof.changes().len();
        ui.separator();
        ui.label(format!("Fix preview for {} · {} accepted/raw text changes. Native/view data is unchanged; revisions and freshness follow the existing edit authority.",fix.proof.issue(),count));
        egui::ScrollArea::both()
            .id_salt("validation-fix-review")
            .max_height(200.)
            .show(ui, |ui| {
                for c in fix.proof.changes().iter().take(128) {
                    ui.label(format!(
                        "Requirement #{} · {}\nBefore: {}\nAfter: {}",
                        c.requirement.0,
                        c.field,
                        text(&c.before),
                        text(&c.after)
                    ));
                }
            });
        if count > 128 {
            ui.colored_label(egui::Color32::DARK_RED,"More than128 changes: Apply unavailable because this review cannot display the complete change set.");
        }
        if !current {
            ui.colored_label(
                egui::Color32::DARK_RED,
                "Fix preview is stale after source/form/scope changes. Run and preview again.",
            );
        }
        ui.horizontal(|ui|{
   if mark(audit,"validation:Apply fix",ui.add_enabled(current&&count<=128&&!blocked(w),egui::Button::new("Apply reviewed fix"))).clicked(){
    match w.board.apply_quick_fix(e,&fix.proof){
     Ok(())=>{w.epoch=w.epoch.saturating_add(1);w.validation.reviewed.fix=None;w.validation.reviewed.note="Fix applied as one register edit. Undo register edit reverses the complete fix; rerun validation for current issues.".into();},
     Err(error)=>w.validation.reviewed.note=error,
    }
   }
   if mark(audit,"validation:Cancel fix",ui.button("Cancel fix preview")).clicked(){w.validation.reviewed.fix=None;}
  });
    }
    if blocked(w) {
        ui.label("Finish register IO/preservation prompts and preserve new requirement fields before reviewing or applying changes.");
    }
    ui.label(&w.validation.reviewed.note);
}
