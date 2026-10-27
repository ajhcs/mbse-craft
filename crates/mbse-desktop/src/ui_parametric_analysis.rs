//! Explicit session-only execution over the existing native parametric authority.
use super::*;
use mbse_model::types::parametrics::analysis::{self as a, Assessment, Program, Run};
#[derive(Default)]
pub(super) struct Analysis {
    pub program: Option<Program>,
    pub report: Option<Run>,
    pub note: String,
}
#[derive(Clone, Copy)]
pub(super) enum AnalysisAction { Prepare, Run }
impl Workspace {
    pub(super) fn pm_analysis_action(&mut self, stamp: &Stamp, action: AnalysisAction) -> Result<(), String> {
        if !self.pm_ready() || self.pm_pending() || self.parametrics.owner.as_ref().is_some_and(|o| !self.pm_owner_matches(o)) || *stamp != self.views.views.stamp(&self.session.editor) {
            return Err("Finish pending work or reselect the current native source before analysis.".into());
        }
        let selected=self.parametrics.specification.ok_or("Select an accepted specification before analysis.")?;
        let result=match action {
            AnalysisAction::Prepare => {
                self.parametrics.analysis.program=None;
                a::prepare(&self.session.editor,selected).map(|p|{
                    self.parametrics.analysis.program=Some(p);
                    self.parametrics.analysis.note="Exact sources prepared. Run explicitly to compute; previous result retained separately.".into();
                })
            }
            AnalysisAction::Run => {
                let p=self.parametrics.analysis.program.as_ref().ok_or("Prepare fresh sources before running.")?;
                if p.specification()!=selected { return Err("Prepared specification differs from selection. Prepare this scope explicitly.".into()); }
                a::evaluate(&self.session.editor,p).map(|report|{
                    self.parametrics.analysis.report=Some(report);
                    self.parametrics.analysis.note="Bounded calculation completed. Numeric/criterion failures stay explicit; no verification or source was written.".into();
                })
            }
        };
        result.map_err(|d|format!("{:?}: {} Previous result retained.",d.code,d.message))
    }
    pub(super) fn pm_analysis_ui(&mut self, ui: &mut egui::Ui, ready: bool) -> Option<AnalysisAction> {
        let selected=self.parametrics.specification;let mut action=None;
        let header=ui.collapsing("Bounded quantity analysis and computed assessment",|ui|{
            ui.label("Prepare exact current bindings, then run the existing native engine. Computed Met/NotMet is separate from verification. Sources, observations and verification remain unchanged.");
            let prepared=self.parametrics.analysis.program.as_ref().map(|p|(p.specification(),p.is_current(&self.session.editor),p.stamp().clone()));
            ui.horizontal_wrapped(|ui|{
                let r=ui.add_enabled(ready&&selected.is_some(),egui::Button::new("Prepare fresh analysis"));self.bound("parametric:analysis:Prepare",&r);if r.clicked(){action=Some(AnalysisAction::Prepare);}
                let r=ui.add_enabled(ready&&prepared.as_ref().is_some_and(|(id,current,_)|*current&&Some(*id)==selected),egui::Button::new("Run bounded calculation"));self.bound("parametric:analysis:Run",&r);if r.clicked(){action=Some(AnalysisAction::Run);}
                let r=ui.add_enabled(self.parametrics.analysis.program.is_some()||self.parametrics.analysis.report.is_some(),egui::Button::new("Clear session analysis"));self.bound("parametric:analysis:Clear",&r);if r.clicked(){self.parametrics.analysis=Analysis::default();}
            });
            if let Some((id,current,stamp))=prepared {ui.colored_label(if current&&Some(id)==selected{ACCENT}else{ERROR},format!("Prepared specification #{} · {} · model r{} · raw r{}",id.0,if !current{"STALE / historical source"}else if Some(id)!=selected{"Different selected scope; prepare explicitly"}else{"Current exact source"},stamp.model_revision,stamp.raw_version));}
            ui.label(&self.parametrics.analysis.note);
            if let Some(report)=self.parametrics.analysis.report.clone(){
                let current=report.is_current(&self.session.editor);let same=Some(report.specification)==selected;
                ui.colored_label(if current&&same{ACCENT}else{ERROR},if !current{"STALE / historical result. This does not describe the current source."}else if !same{"Result belongs to a different selected specification."}else{"Result matches this exact source and selected specification."});
                ui.label(format!("Specification #{} · calculation #{} · document {} · model r{} · raw r{}",report.specification.0,report.calculation.0,report.stamp.document_id.0,report.stamp.model_revision,report.stamp.raw_version));
                match &report.outcome {mbse_model::stage1_calc::Outcome::Value(v)=>{let r=ui.heading(format!("Computed quantity: {} · {}",v.value,quantity_text(v.quantity)));self.bound("parametric:analysis:Quantity",&r);},mbse_model::stage1_calc::Outcome::Failure(d)=>{let r=ui.colored_label(ERROR,format!("Numeric failure · {:?}: {}",d.code,d.message));self.bound("parametric:analysis:Numeric failure",&r);}}
                let text=match &report.assessment {Assessment::Met=>"Computed Met · verification unchanged".into(),Assessment::NotMet=>"Computed NotMet · verification unchanged".into(),Assessment::Unavailable(d)=>format!("Assessment unavailable · {:?}: {}",d.code,d.message)};
                let r=ui.label(text);self.bound("parametric:analysis:Assessment",&r);
                if let Ok(c)=&report.criterion {ui.label(format!("Native criterion: {} {} · {}",c.threshold.decimal,quantity_text(c.threshold.quantity),c.comparator));}
                ui.label(format!("Rules {} · native algorithm {} · unit table {}",report.rules_revision,report.native_receipt.algorithm_revision,report.native_receipt.unit_table_revision));
                ui.label(format!("Source SHA256 {}",report.stamp.snapshot_hash));
                let details=ui.collapsing("Frozen sources and diagnostics",|ui|{
                    for b in &report.bindings {ui.label(format!("Binding #{} / parameter #{} / {} · {} · {:?} · origin {:?}",b.binding.0,b.parameter.0,b.input_name,quantity_text(b.declared),b.source,b.value_origin));}
                    for (id,outcome) in &report.native_receipt.outcomes {if let mbse_model::stage1_calc::Outcome::Failure(d)=outcome {ui.colored_label(ERROR,format!("Calculation #{} · {:?}: {}",id.0,d.code,d.message));}}
                    for limit in &report.limits {ui.label(limit);}
                });self.bound("parametric:analysis:Details",&details.header_response);
                let r=ui.button("Copy historical analysis JSON");self.bound("parametric:analysis:Copy",&r);if r.clicked(){ui.ctx().copy_text(serde_json::to_string_pretty(&report).expect("finite result serializes"));}
                ui.label("Session result only. Save/reopen retains source data, not a prepared program or live result. A copied report cannot execute or establish verification. Native criterion units: metre/one/percent; typed expression scratch is not executed. No general solver or engineering acceptance.");
            }
        });self.bound("parametric:Analysis controls",&header.header_response);action
    }
    pub(super) fn pm_analysis_audit(&self)->serde_json::Value {
        let p=&self.parametrics.analysis;serde_json::json!({"program":p.program.as_ref().map(|p|serde_json::json!({"specification":p.specification(),"stamp":p.stamp(),"current":p.is_current(&self.session.editor),"selected_scope":Some(p.specification())==self.parametrics.specification})),"report":p.report,"current":p.report.as_ref().map(|r|r.is_current(&self.session.editor)),"selected_scope":p.report.as_ref().map(|r|Some(r.specification)==self.parametrics.specification),"note":p.note})
    }
}
