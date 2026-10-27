//! Sealed append-only caller-declared component payloads, through existing model authority.
//! The source adapter must separately verify bytes, supported semantics and loss review.
use super::*;
use crate::stage1::{Editor,Freshness};
pub const RULE:&str="mbse-craft.component-admission/1";
pub const MAX_IDENTITIES:usize=64;
pub const MAX_REQUEST_BYTES:usize=128*1024;
pub const PROVENANCE_NAMESPACE:&str="urn:mbse-craft:source-correspondence";
pub const BLOCK_NAMESPACE:&str="http://www.omg.org/spec/SysML/20181001/SysML";
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct Source {pub member:String,pub primary_sha256:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct BlockDeclaration {pub source_id:String,pub namespace:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct Component {pub source_id:String,pub name:String,pub block:Option<BlockDeclaration>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct ComponentUsage {pub source_id:String,pub name:String,pub owner:String,pub target:String,pub kind:UsageKind}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct Payload {pub model_id:String,pub model_name:String,pub components:Vec<Component>,pub usages:Vec<ComponentUsage>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct Correspondence {pub source_id:String,pub source_kind:String,pub native:Id,pub native_kind:tags::TargetKind}
#[derive(Clone,Debug,Serialize)]
pub struct Summary {pub rule:String,pub source:Source,pub system:RecordId,pub correspondences:Vec<Correspondence>,pub created_components:usize,pub created_usages:usize,pub declared_blocks:usize,pub limits:Vec<String>}
/// Not deserializable or editable. Candidate state and source correspondence
/// metadata cannot be substituted after preparation.
#[derive(Clone,Debug)]
pub struct Plan {native:Freshness,digest:String,candidate:Snapshot,summary:Summary}
impl Plan {
 pub fn summary(&self)->&Summary{&self.summary}
 pub fn check(&self,e:&Editor)->Result<(),String>{if self.native!=e.freshness()||self.digest!=e.snapshot.digest(){return Err("ADMISSION_PLAN_STALE_OR_FOREIGN_RUNTIME".into());}Ok(())}
}
fn profile()->tags::Profile {
 let fields=["member","primary_sha256","source_id","source_kind","source_name","source_namespace","model_id","model_name","adapter_rule"].into_iter().map(|s|(s.into(),Scalar::Text)).collect::<BTreeMap<_,_>>();
 tags::Profile{identity:tags::Identity{namespace:PROVENANCE_NAMESPACE.into(),version:"1".into()},stereotypes:BTreeMap::from([
  ("Element".into(),tags::Stereotype{applies_to:[tags::TargetKind::ComponentDefinition,tags::TargetKind::Part,tags::TargetKind::Reference].into_iter().collect(),tags:fields.clone()}),
  ("BlockDeclaration".into(),tags::Stereotype{applies_to:[tags::TargetKind::ComponentDefinition].into_iter().collect(),tags:fields}),
 ])}
}
fn text(s:&str,max:usize)->bool{s.len()<=max&&!s.chars().any(char::is_control)}
fn source_id(s:&str)->bool{!s.is_empty()&&text(s,256)&&!s.chars().any(char::is_whitespace)}
fn metadata(store:&mut Store,source:&Source,payload:&Payload,source_id:&str,source_kind:&str,source_name:&str,namespace:&str,target:Id,expected:tags::TargetKind,stereotype:&str)->Result<(),String>{
 // These literals are declarations, not proof of original source validity,
 // verification or metamodel equivalence. No imported object is reused by name.
 for old in store.profile_applications.values().filter(|a|a.profile.namespace==PROVENANCE_NAMESPACE){
  if ["member","primary_sha256","source_id"].into_iter().zip([source.member.as_str(),source.primary_sha256.as_str(),source_id]).all(|(key,value)|old.values.get(key).is_some_and(|v|v.kind==Scalar::Text&&v.text==value)){return Err("ADMISSION_SOURCE_ID_ALREADY_ATTACHED".into());}
 }
 let values=[("member",source.member.as_str()),("primary_sha256",source.primary_sha256.as_str()),("source_id",source_id),("source_kind",source_kind),("source_name",source_name),("source_namespace",namespace),("model_id",payload.model_id.as_str()),("model_name",payload.model_name.as_str()),("adapter_rule",RULE)].into_iter().map(|(k,v)|(k.into(),values::Literal{kind:Scalar::Text,text:v.into()})).collect();
 let a=tags::Application{target,expected,profile:profile().identity,stereotype:stereotype.into(),values};
 if store.profile_applications.insert(a.key(),a).is_some(){return Err("ADMISSION_APPLICATION_COLLISION".into());}Ok(())
}
pub fn prepare(e:&Editor,system:RecordId,source:Source,payload:Payload)->Result<Plan,String>{
 e.snapshot.validate()?;
 if source.member.is_empty()||source.member.len()>64||!source.member.bytes().all(|c|c.is_ascii_alphanumeric()||b"_-".contains(&c))||source.primary_sha256.len()!=64||!source.primary_sha256.bytes().all(|c|c.is_ascii_digit()||(b'a'..=b'f').contains(&c)){return Err("ADMISSION_EXACT_SOURCE_CONTEXT_REQUIRED".into());}
 let count=1usize.checked_add(payload.components.len()).and_then(|n|n.checked_add(payload.usages.len())).and_then(|n|n.checked_add(payload.components.iter().filter(|c|c.block.is_some()).count())).ok_or("ADMISSION_IDENTITY_LIMIT")?;
 if count>MAX_IDENTITIES||payload.components.is_empty(){return Err("ADMISSION_IDENTITY_LIMIT_OR_EMPTY_COMPONENTS".into());}
 if !text(&payload.model_name,256)||!text(&payload.model_id,256)||payload.components.iter().any(|c|!text(&c.name,256)||!text(&c.source_id,256)||c.block.as_ref().is_some_and(|b|!text(&b.source_id,256)||!text(&b.namespace,256)))||payload.usages.iter().any(|u|[&u.name,&u.source_id,&u.owner,&u.target].iter().any(|v|!text(v,256))) {return Err("ADMISSION_REQUEST_TEXT_LIMIT".into());}
 if serde_json::to_vec(&(&source,&payload)).map_err(|e|e.to_string())?.len()>MAX_REQUEST_BYTES{return Err("ADMISSION_REQUEST_BUDGET".into());}
 let mut ids=BTreeSet::new();
 for id in std::iter::once(&payload.model_id).chain(payload.components.iter().map(|c|&c.source_id)).chain(payload.usages.iter().map(|u|&u.source_id)).chain(payload.components.iter().filter_map(|c|c.block.as_ref().map(|b|&b.source_id))){if !source_id(id)||!ids.insert(id.clone()){return Err("ADMISSION_UNIQUE_EXACT_SOURCE_ID_REQUIRED".into());}}
 if e.snapshot.records.get(&system).is_none_or(|r|r.kind!=RecordKind::System){return Err("ADMISSION_EXISTING_SYSTEM_REQUIRED".into());}
 let mut candidate=e.snapshot.clone();let mut store=load(&candidate)?;let declared=profile();
 if store.profiles.get(&declared.identity.key()).is_some_and(|old|old!=&declared){return Err("ADMISSION_PROVENANCE_PROFILE_COLLISION".into());}store.profiles.insert(declared.identity.key(),declared);
 let mut mappings=vec![Correspondence{source_id:payload.model_id.clone(),source_kind:"UmlModel".into(),native:Id(system.0),native_kind:tags::TargetKind::System}];
 const UML:&str="http://www.omg.org/spec/UML/20131001";
 let mut components=BTreeMap::new();
 for c in &payload.components{
  name(&c.name)?;let id=allocate(&mut candidate)?;components.insert(c.source_id.clone(),id);store.definitions.insert(id,Definition{id,system,name:c.name.clone(),kind:Kind::Component});
  metadata(&mut store,&source,&payload,&c.source_id,"UmlClass",&c.name,UML,id,tags::TargetKind::ComponentDefinition,"Element")?;
  mappings.push(Correspondence{source_id:c.source_id.clone(),source_kind:"UmlClass".into(),native:id,native_kind:tags::TargetKind::ComponentDefinition});
  if let Some(block)=&c.block{if block.namespace!=BLOCK_NAMESPACE{return Err("ADMISSION_BLOCK_DECLARATION_NAMESPACE_UNSUPPORTED".into());}metadata(&mut store,&source,&payload,&block.source_id,"SysmlBlockDeclaration","Block",&block.namespace,id,tags::TargetKind::ComponentDefinition,"BlockDeclaration")?;mappings.push(Correspondence{source_id:block.source_id.clone(),source_kind:"SysmlBlockDeclaration".into(),native:id,native_kind:tags::TargetKind::ComponentDefinition});}
 }
 for u in &payload.usages{
  name(&u.name)?;let owner=*components.get(&u.owner).ok_or("ADMISSION_PAYLOAD_OWNER_UNRESOLVED")?;let target=*components.get(&u.target).ok_or("ADMISSION_PAYLOAD_TARGET_UNRESOLVED")?;
  let id=allocate(&mut candidate)?;let native_kind=if u.kind==UsageKind::Part{tags::TargetKind::Part}else{tags::TargetKind::Reference};let source_kind=if u.kind==UsageKind::Part{"UmlCompositeProperty"}else{"UmlReferenceProperty"};
  store.usages.insert(id,Usage{id,name:u.name.clone(),owner,target,kind:u.kind});metadata(&mut store,&source,&payload,&u.source_id,source_kind,&u.name,UML,id,native_kind,"Element")?;mappings.push(Correspondence{source_id:u.source_id.clone(),source_kind:source_kind.into(),native:id,native_kind});
 }
 store.validate()?;put(&mut candidate,NAMESPACE,&store)?;candidate.validate()?;
 Ok(Plan{native:e.freshness(),digest:e.snapshot.digest(),candidate,summary:Summary{rule:RULE.into(),source,system,correspondences:mappings,created_components:payload.components.len(),created_usages:payload.usages.len(),declared_blocks:payload.components.iter().filter(|c|c.block.is_some()).count(),limits:vec!["Caller-declared payload only. Source verification and strict semantic/loss review belong to the source adapter; no XML or file reads here.".into(),"Model container maps explicitly to the selected existing System without renaming it. Classes use existing component definitions; one-to-one Part/Reference usages use the existing native type authority.".into(),"Source and Block correspondence tags are documentary declarations, not full SysML semantics, engineering verification or source truth. No evaluation, layout import or ambient acquisition.".into(),"Atomic single model-history append; existing meaning, invalid raw journals and opaque fields retained. Maximum64source identities/128KiBrequest, plus existing global type/profile/allocator limits.".into()]}})
}
pub(super) fn apply_inner(snapshot:&mut Snapshot,plan:Plan)->Result<Change,String>{
 if plan.digest!=snapshot.digest(){return Err("ADMISSION_PLAN_STALE".into());}plan.candidate.validate()?;*snapshot=plan.candidate;Ok(Change::Model)
}
