//! Source-stamped accepted-data counts. No calculation, adequacy or verification execution.
use super::*;
use mbse_model::{stage1::{RecordId,RecordKind},types::{self as ty,Id}};
pub const RULES_REVISION:&str="mbse-craft.scoped-metrics/1";
pub const PROFILE:&str="mbse-craft.scoped-metrics-report/1";
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeScope { All, System(RecordId) }
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RequirementScope { All, Selected(BTreeSet<RequirementId>) }
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope { pub native:NativeScope,pub requirements:RequirementScope }
impl Default for Scope {fn default()->Self{Self{native:NativeScope::All,requirements:RequirementScope::All}}}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct RegisterSource {pub revision:u64,pub sha256:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct ModelCounts {
 pub top_level_records:usize,
 pub native_by_kind:BTreeMap<String,usize>,
 pub calculation_definitions:usize,
 pub typed:Option<BTreeMap<String,usize>>,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct TraceCounts {
 pub requirements:usize,
 pub selected_ids:Vec<RequirementId>,
 pub parent_links_within_scope:usize,
 pub parent_links_crossing_scope:usize,
 pub traces:usize,
 pub active_traces:usize,
 pub detached_traces:usize,
 pub boundary_traces:usize,
 pub by_kind:BTreeMap<String,usize>,
 pub documentary_test_cases_referenced:usize,
 pub evidence_records:usize,
 pub declared_maturity:BTreeMap<String,usize>,
 pub declared_implementation:BTreeMap<String,usize>,
 pub declared_verification:BTreeMap<String,usize>,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct Report {
 pub profile:String,
 pub rules_revision:String,
 pub native_source:DocumentStamp,
 pub register_source:Option<RegisterSource>,
 pub scope:Scope,
 pub model:ModelCounts,
 pub traces:Option<TraceCounts>,
 pub opaque_native_namespaces:Vec<String>,
 pub limits:Vec<String>,
}
impl Report {
 pub fn is_current(&self,editor:&Editor,board:Option<&Board>,scope:&Scope)->bool{
  self.profile==PROFILE&&self.rules_revision==RULES_REVISION&&self.scope==*scope&&self.native_source==editor.calculation_stamp()&&register_source(board).is_ok_and(|source|source==self.register_source)
 }
 pub fn encoded(&self)->Result<Vec<u8>,String>{serde_json::to_vec_pretty(self).map_err(|e|e.to_string())}
}
fn register_source(board:Option<&Board>)->Result<Option<RegisterSource>,String>{board.map(|b|digest(&b.register).map(|sha256|RegisterSource{revision:b.register.revision,sha256})).transpose()}
fn bump(counts:&mut BTreeMap<String,usize>,key:impl Into<String>){*counts.entry(key.into()).or_default()+=1;}
fn named<T:std::fmt::Debug>(counts:&mut BTreeMap<String,usize>,value:T){bump(counts,format!("{value:?}"));}
fn model_counts(e:&Editor,store:Option<&ty::Store>,scope:&NativeScope)->ModelCounts{
 let owns=|system:RecordId|matches!(scope,NativeScope::All)||*scope==NativeScope::System(system);
 let mut native_by_kind=["System","Mission","Capability","Objective","Measure","Stakeholder","Expectation","Environment","Organization","Actor"].into_iter().map(|k|(k.into(),0)).collect();
 let mut top_level_records=0;let mut calculation_definitions=0;
 for r in e.snapshot.records.values().filter(|r|owns(r.scope)){top_level_records+=1;named(&mut native_by_kind,r.kind);if r.fields.get("calculation").is_some_and(|v|!v.is_null()){calculation_definitions+=1;}}
 let Some(s)=store else{return ModelCounts{top_level_records,native_by_kind,calculation_definitions,typed:None};};
 let mut typed=["component_definitions","value_definitions","properties","part_usages","reference_usages","interfaces","ports","connectors","activities","activity_partitions","activity_nodes","activity_pins","activity_flows","state_machines","states","events","transitions","scenarios","lifelines","messages","alternatives","operands","parametric_specifications","parameters","bindings","default_literal_entries","override_literal_entries"].into_iter().map(|k|(k.into(),0)).collect::<BTreeMap<String,usize>>();
 let definition=|id:Id|s.definitions.get(&id).is_some_and(|d|owns(d.system));
 for d in s.definitions.values().filter(|d|owns(d.system)){bump(&mut typed,if d.kind==ty::Kind::Component{"component_definitions"}else{"value_definitions"});}
 typed.insert("properties".into(),s.properties.values().filter(|p|definition(p.owner)).count());
 for u in s.usages.values().filter(|u|definition(u.owner)){bump(&mut typed,if u.kind==ty::UsageKind::Part{"part_usages"}else{"reference_usages"});}
 typed.insert("interfaces".into(),s.interfaces.values().filter(|i|owns(i.system)).count());
 typed.insert("ports".into(),s.ports.values().filter(|p|definition(p.owner)).count());
 typed.insert("connectors".into(),s.connectors.values().filter(|c|definition(c.owner)).count());
 for a in s.activities.values().filter(|a|owns(a.system)){bump(&mut typed,"activities");for(key,n)in[("activity_partitions",a.partitions.len()),("activity_nodes",a.nodes.len()),("activity_pins",a.pins.len()),("activity_flows",a.flows.len())]{*typed.get_mut(key).unwrap()+=n;}}
 for m in s.state_machines.values().filter(|m|owns(m.system)){bump(&mut typed,"state_machines");for(key,n)in[("states",m.states.len()),("events",m.events.len()),("transitions",m.transitions.len())]{*typed.get_mut(key).unwrap()+=n;}}
 for a in s.scenarios.values().filter(|a|owns(a.system)){bump(&mut typed,"scenarios");for(key,n)in[("lifelines",a.lifelines.len()),("messages",a.messages.len()),("alternatives",a.alternatives.len()),("operands",a.alternatives.values().map(|a|a.operands.len()).sum())]{*typed.get_mut(key).unwrap()+=n;}}
 for a in s.parametrics.values().filter(|a|owns(a.system)){bump(&mut typed,"parametric_specifications");*typed.get_mut("parameters").unwrap()+=a.parameters.len();*typed.get_mut("bindings").unwrap()+=a.bindings.len();}
 typed.insert("default_literal_entries".into(),s.defaults.keys().filter(|id|s.properties.get(id).is_some_and(|p|definition(p.owner))).count());
 typed.insert("override_literal_entries".into(),s.overrides.iter().filter(|(id,_)|s.usages.get(id).is_some_and(|u|definition(u.owner))).map(|(_,values)|values.len()).sum());
 ModelCounts{top_level_records,native_by_kind,calculation_definitions,typed:Some(typed)}
}
fn trace_counts(e:&Editor,store:Option<&ty::Store>,b:&Board,scope:&RequirementScope)->Result<TraceCounts,String>{
 let selected=match scope{RequirementScope::All=>b.register.requirements.keys().copied().collect(),RequirementScope::Selected(ids)=>{if ids.len()>10000{return Err("METRICS_REQUIREMENT_SCOPE_LIMIT".into());}if ids.iter().any(|id|!b.register.requirements.contains_key(id)){return Err("METRICS_REQUIREMENT_SCOPE_MISSING".into());}ids.clone()}};
 let mut out=TraceCounts{requirements:selected.len(),selected_ids:selected.iter().copied().collect(),parent_links_within_scope:0,parent_links_crossing_scope:0,traces:0,active_traces:0,detached_traces:0,boundary_traces:0,by_kind:["Derive","Refine","Satisfy","Verify","Copy"].into_iter().map(|k|(k.into(),0)).collect(),documentary_test_cases_referenced:0,evidence_records:0,declared_maturity:BTreeMap::new(),declared_implementation:BTreeMap::new(),declared_verification:BTreeMap::new()};
 for id in &selected{let r=&b.register.requirements[id];named(&mut out.declared_maturity,r.maturity);named(&mut out.declared_implementation,r.implementation);named(&mut out.declared_verification,r.verification);if let Some(parent)=r.parent{if selected.contains(&parent){out.parent_links_within_scope+=1;}else{out.parent_links_crossing_scope+=1;}}}
 let legacy_projection=if store.is_none(){b.trace_projection(&e.snapshot).into_iter().map(|p|(p.trace,p.active)).collect::<BTreeMap<_,_>>()}else{BTreeMap::new()};
 let mut cases=BTreeSet::new();
 for t in b.register.traces.values(){
  let target=selected.contains(&t.target);let source=match t.source{TraceEndpoint::Requirement(id)=>Some(selected.contains(&id)),_=>None};
  if source.is_some_and(|source|source!=target){out.boundary_traces+=1;continue;}
  if !target{continue;}
  out.traces+=1;named(&mut out.by_kind,t.kind);
  if store.map(|s|traces::project_with_store(&e.snapshot,s,&b.register,t).active).unwrap_or_else(||legacy_projection[&t.id]){out.active_traces+=1;}else{out.detached_traces+=1;}
  if let TraceEndpoint::TestCase(id)=t.source{cases.insert(id);}
 }
 out.documentary_test_cases_referenced=cases.len();out.evidence_records=b.register.evidence.values().filter(|v|selected.contains(&v.requirement)).count();Ok(out)
}
/// Model and requirement scopes are independent; no inferred System ownership of requirements.
pub fn query(e:&Editor,board:Option<&Board>,scope:Scope)->Result<Report,String>{
 e.snapshot.validate()?;let store=e.snapshot.extensions.as_ref().map(|_|ty::load(&e.snapshot)).transpose()?;
 if let NativeScope::System(id)=scope.native{if e.snapshot.records.get(&id).is_none_or(|r|r.kind!=RecordKind::System){return Err("METRICS_NATIVE_SYSTEM_SCOPE_MISSING_OR_WRONG_KIND".into());}}
 if let Some(b)=board{validate_register(&b.register)?;envelope_size(&b.register)?;}else if !matches!(scope.requirements,RequirementScope::All){return Err("METRICS_REGISTER_REQUIRED_FOR_SELECTION".into());}
 let model=model_counts(e,store.as_ref(),&scope.native);let traces=board.map(|b|trace_counts(e,store.as_ref(),b,&scope.requirements)).transpose()?;
 let opaque_native_namespaces=e.snapshot.extensions.as_ref().map(|x|x.namespaces.keys().filter(|k|!matches!(k.as_str(),ty::NAMESPACE|ty::DRAFTS)).cloned().collect()).unwrap_or_default();
 Ok(Report{profile:PROFILE.into(),rules_revision:RULES_REVISION.into(),native_source:e.calculation_stamp(),register_source:register_source(board)?,scope,model,traces,opaque_native_namespaces,limits:vec!["Accepted-data inventory only. Native counts are top-level records; embedded actor/context items and presentation geometry are not counted. Calculation definitions overlap their native Measure owners; table categories are not one additive total.".into(),"Model scope and requirement-register scope are independent. Requirement subsets are exact IDs with no automatic descendant expansion; links crossing a requirement subset are reported separately, not included as internal traces.".into(),"Active/detached means the existing trace endpoint projection only. Declared lifecycle counts are not current verification, fulfillment, evidence adequacy or engineering acceptance. No calculations or verification run.".into(),"Legacy documents without an extension remain unchanged: native counts are available, typed metrics are unavailable (null), and existing legacy trace projection rules remain unchanged. Typed counts cover declared schema families and accepted literal entries; profiles/applications/opaque namespace contents, raw drafts and UI occurrences are not inventoried. Unsupported/malformed source refuses rather than substituting zero.".into(),"No provided register means trace metrics unavailable, distinct from an explicitly provided empty register. Source-stamped JSON export is documentary; it cannot execute or authorize verification.".into()]})
}
