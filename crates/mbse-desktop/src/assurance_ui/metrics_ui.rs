//! Accepted-data counts in the existing independent-register window.
use super::*;
use mbse_assurance::metrics::{self as m,NativeScope,RequirementScope,Scope,Report};
use mbse_model::stage1::RecordId;
use std::collections::BTreeSet;
pub(super) struct State {
 pub native_all:bool,pub system:String,pub include_register:bool,pub requirements_all:bool,pub ids:String,pub report:Option<Report>,pub note:String,
}
impl Default for State {fn default()->Self{Self{native_all:true,system:"1".into(),include_register:true,requirements_all:true,ids:String::new(),report:None,note:String::new()}}}
fn id(raw:&str)->Result<u64,String>{let id=raw.parse::<u64>().map_err(|_|"Use exact positive decimal IDs.".to_string())?;if id==0||id.to_string()!=raw{return Err("Use exact positive decimal IDs without aliases or leading zeroes.".into());}Ok(id)}
fn scope(state:&State)->Result<Scope,String>{
 let native=if state.native_all{NativeScope::All}else{NativeScope::System(RecordId(id(&state.system)?))};
 let requirements=if !state.include_register||state.requirements_all{RequirementScope::All}else{
  if state.ids.len()>210000{return Err("Requirement scope exceeds210000 bytes.".into());}let mut ids=BTreeSet::new();
  for token in state.ids.split(|c:char|c==','||c.is_whitespace()).filter(|s|!s.is_empty()){if !ids.insert(RequirementId(id(token)?)){return Err("Duplicate requirement IDs are not accepted; original scope text retained.".into());}if ids.len()>10000{return Err("Select at most10000 exact requirement IDs.".into());}}
  RequirementScope::Selected(ids)
 };Ok(Scope{native,requirements})
}
fn ready(w:&AssuranceWorkspace)->bool{w.pending.is_none()&&!w.close_prompt&&!w.replace_prompt&&!w.unadmitted_form()}
fn current(w:&AssuranceWorkspace,e:&Editor)->bool{ready(w)&&scope(&w.metrics).is_ok_and(|scope|w.metrics.report.as_ref().is_some_and(|r|r.is_current(e,w.metrics.include_register.then_some(&w.board),&scope)))}
pub(super) fn audit(w:&AssuranceWorkspace,e:&Editor)->serde_json::Value{serde_json::json!({"ready":ready(w),"current":current(w,e),"native_all":w.metrics.native_all,"system":w.metrics.system,"include_register":w.metrics.include_register,"requirements_all":w.metrics.requirements_all,"ids":w.metrics.ids,"requested_scope":scope(&w.metrics).ok(),"scope_error":scope(&w.metrics).err(),"report":w.metrics.report,"note":w.metrics.note})}
pub(super) fn show(w:&mut AssuranceWorkspace,ui:&mut egui::Ui,e:&Editor,audit:&mut BTreeMap<String,[f32;4]>){
 let section=ui.collapsing("Scoped model and trace metrics",|ui|{
  ui.label("Accepted-data inventory. Model and requirement scopes are independent; no inferred System ownership of requirements. Counts and declared lifecycle states do not establish fulfillment, verification or evidence adequacy. This query does not execute calculations.");
  ui.horizontal_wrapped(|ui|{ui.label("Native model scope");mark(audit,"metrics:Native all",ui.selectable_value(&mut w.metrics.native_all,true,"All native Systems"));mark(audit,"metrics:Native system",ui.selectable_value(&mut w.metrics.native_all,false,"Exact System ID"));mark(audit,"metrics:system",ui.add_enabled(!w.metrics.native_all,egui::TextEdit::singleline(&mut w.metrics.system).char_limit(20).desired_width(150.).id(egui::Id::new("metrics-system-id"))));});
  let systems=e.snapshot.records.values().filter(|r|r.kind==mbse_model::stage1::RecordKind::System).collect::<Vec<_>>();let picker=ui.collapsing(format!("Existing native Systems ({})",systems.len()),|ui|{for r in systems{if mark(audit,format!("metrics:system:{}",r.id.0),ui.button(format!("System #{} · {}",r.id.0,r.name))).clicked(){w.metrics.native_all=false;w.metrics.system=r.id.0.to_string();}}});mark(audit,"metrics:Systems",picker.header_response);
  mark(audit,"metrics:Include register",ui.checkbox(&mut w.metrics.include_register,"Include current working requirements register (including unsaved accepted edits)"));
  ui.add_enabled_ui(w.metrics.include_register,|ui|{ui.horizontal_wrapped(|ui|{ui.label("Requirement scope");mark(audit,"metrics:Requirements all",ui.selectable_value(&mut w.metrics.requirements_all,true,"All working requirements"));mark(audit,"metrics:Requirements selected",ui.selectable_value(&mut w.metrics.requirements_all,false,"Exact local IDs"));});mark(audit,"metrics:ids",ui.add_enabled(!w.metrics.requirements_all,egui::TextEdit::multiline(&mut w.metrics.ids).desired_rows(2).char_limit(210000).desired_width(700.).hint_text("1, 2 · empty means an explicit empty subset").id(egui::Id::new("metrics-requirement-ids"))));});
  if !w.metrics.include_register{ui.label("Trace metrics unavailable: working register explicitly excluded. Model counts remain available.");}
  let parsed=scope(&w.metrics);if let Err(error)=&parsed{ui.colored_label(egui::Color32::DARK_RED,error);}
  ui.horizontal_wrapped(|ui|{
   let r=mark(audit,"metrics:Query",ui.add_enabled(ready(w)&&parsed.is_ok(),egui::Button::new("Query current accepted data")));
   if r.clicked(){match parsed.clone().and_then(|scope|m::query(e,w.metrics.include_register.then_some(&w.board),scope)){Ok(report)=>{w.metrics.report=Some(report);w.metrics.note="Read-only query complete; native data, register and verification unchanged.".into();},Err(error)=>w.metrics.note=format!("{error} Previous report retained as historical.")}}
   let r=mark(audit,"metrics:Clear",ui.add_enabled(w.metrics.report.is_some(),egui::Button::new("Clear session report")));if r.clicked(){w.metrics.report=None;w.metrics.note="Session report cleared; scope text and source data retained.".into();}
  });
  if !ready(w){ui.label("Finish pending register work or admit the retained form text before querying. Previous reports remain copyable.");}ui.label(&w.metrics.note);
  let Some(report)=w.metrics.report.clone()else{ui.label("No metrics report in this session.");return;};
  ui.colored_label(if current(w,e){egui::Color32::DARK_GREEN}else{egui::Color32::DARK_RED},if current(w,e){"Report matches this exact source and the requested scopes."}else{"STALE / historical report. Source, register, scope or pending work changed; query again for current counts."});
  let native_scope=match &report.scope.native{NativeScope::All=>"All native Systems".into(),NativeScope::System(id)=>format!("System #{}",id.0)};
  let requirement_scope=match &report.scope.requirements{RequirementScope::All=>"All working requirements".into(),RequirementScope::Selected(ids)=>format!("Exact local IDs ({} total; up to16 shown): [{}]",ids.len(),ids.iter().take(16).map(|id|id.0.to_string()).collect::<Vec<_>>().join(", "))};
  ui.label(format!("Reported model scope: {native_scope}; requirement scope: {requirement_scope}"));
  mark(audit,"metrics:count:native",ui.heading(format!("{} native top-level records · {} calculation definitions",report.model.top_level_records,report.model.calculation_definitions)));
  let native=ui.collapsing("Native record-kind counts",|ui|{for(kind,count)in &report.model.native_by_kind{ui.label(format!("{kind}: {count}"));}});mark(audit,"metrics:Native counts",native.header_response);
  if let Some(counts)=&report.model.typed{let typed=ui.collapsing(format!("Typed inventory ({} categories; not an additive total)",counts.len()),|ui|{for(kind,count)in counts{ui.label(format!("{}: {count}",kind.replace('_'," ")));}});mark(audit,"metrics:Typed counts",typed.header_response);}else{mark(audit,"metrics:typed unavailable",ui.label("Typed metrics unavailable for this legacy document; no migration or empty substitute was written."));}
  if let Some(traces)=&report.traces{
   mark(audit,"metrics:count:traces",ui.heading(format!("{} requirements · {} included traces · {} active · {} detached · {} boundary traces",traces.requirements,traces.traces,traces.active_traces,traces.detached_traces,traces.boundary_traces)));
   ui.label(format!("Parent edges: {} within scope · {} crossing either direction. {} referenced documentary test cases · {} evidence records.",traces.parent_links_within_scope,traces.parent_links_crossing_scope,traces.documentary_test_cases_referenced,traces.evidence_records));
   let kinds=ui.collapsing("Trace kinds and declared lifecycle counts",|ui|{for(kind,count)in &traces.by_kind{ui.label(format!("{kind}: {count}"));}for(label,counts)in[("Declared maturity",&traces.declared_maturity),("Declared implementation",&traces.declared_implementation),("Declared verification",&traces.declared_verification)]{ui.label(format!("{label}: {counts:?}"));}});mark(audit,"metrics:Trace counts",kinds.header_response);
  }else{ui.label("Reported trace metrics unavailable: no register was provided to this query.");}
  ui.label(format!("Rules {} · native document {} · model r{} · raw r{}",report.rules_revision,report.native_source.document_id,report.native_source.model_revision,report.native_source.raw_version));ui.label(format!("Native source SHA256 {}",report.native_source.snapshot_hash));
  if let Some(source)=&report.register_source{ui.label(format!("Working register r{} · SHA256 {}",source.revision,source.sha256));}
  let details=ui.collapsing("Inventory limits and opaque namespace names",|ui|{for limit in &report.limits{ui.label(limit);}ui.label(format!("Opaque namespace names (contents not inventoried): {:?}",report.opaque_native_namespaces));});mark(audit,"metrics:Limits",details.header_response);
  if mark(audit,"metrics:Copy",ui.button("Copy historical metrics JSON")).clicked(){match report.encoded(){Ok(bytes)=>ui.ctx().copy_text(String::from_utf8(bytes).expect("JSON UTF8")),Err(error)=>w.metrics.note=error}}
  ui.label("Session report only. Cold reopen does not restore a report. Copied JSON retains reported scope, rule version, revisions and hashes; it cannot execute or establish verification.");
 });mark(audit,"metrics:section",section.header_response);
}
#[cfg(test)]
#[path="metrics_tests.rs"]
mod tests;
