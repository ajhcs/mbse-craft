//! Strict, whole-member component subset. Existing projection identities and
//! native type/history authority are reused; unsupported content never drops silently.
use crate::{project_sets::{ProjectSet,FileState,Format,ElementKey,LocalIdentity,Recheck},v2};
use mbse_model::{stage1::{Editor,RecordId,Command},types::{self as types,admission as native}};
use serde::Serialize;
use std::{collections::{BTreeMap,BTreeSet},sync::Arc};
pub const RULE:&str="mbse-craft.uml-component-source/1";
pub const UML:&str="http://www.omg.org/spec/UML/20131001";
pub const XMI:&str="http://www.omg.org/spec/XMI/20131001";
pub const MAX_XML_BYTES:usize=1024*1024;
pub const MAX_XML_ELEMENTS:usize=512;
pub const MAX_NOTICES:usize=128;
pub const MAX_SUMMARY_BYTES:usize=2*1024*1024;
#[derive(Clone,Debug,Serialize)]
pub struct Notice {pub code:String,pub detail:String,pub blocking:bool,pub start:Option<usize>,pub end:Option<usize>}
#[derive(Clone,Debug,Serialize)]
pub struct Mapping {pub source:ElementKey,pub native:types::Id,pub native_kind:types::tags::TargetKind,pub source_kind:String}
#[derive(Clone,Debug,Serialize)]
pub struct Summary {
 pub rule:String,pub source_set_digest:String,pub selected_member:String,pub source_primary_sha256:Option<String>,pub system:RecordId,
 pub target_native:mbse_model::stage1::Freshness,pub target_snapshot_sha256:String,
 pub ready_for_review:bool,pub required_acknowledgements:BTreeSet<String>,pub notices:Vec<Notice>,pub omitted_notices:usize,
 pub inspection_loss_count:usize,pub inspection_diagnostic_count:usize,pub inspection_details_omitted_budget:bool,
 pub source_inspection_stage:Option<v2::ProjectionStage>,pub source_inspection_losses:Vec<v2::Loss>,pub source_inspection_diagnostics:Vec<v2::Diagnostic>,
 pub mappings:Vec<Mapping>,pub native_candidate:Option<native::Summary>,pub represented_multiplicity_literals:usize,pub limits:Vec<String>,
}
pub struct Prepared {source:Arc<ProjectSet>,summary:Summary,plan:Option<native::Plan>}
pub struct Reviewed {source:Arc<ProjectSet>,summary:Summary,plan:native::Plan}
#[derive(Clone,Debug,Serialize)]
pub struct Applied {pub summary:Summary,pub original_file_check:Recheck,pub checked_unix_milliseconds:u128,pub target_model_revision:u64,pub target_snapshot_sha256:String}
impl Prepared {
 pub fn summary(&self)->&Summary{&self.summary}
 pub fn target_current(&self,e:&Editor)->bool{self.plan.as_ref().is_some_and(|p|p.check(e).is_ok())}
 pub fn review(&self,acknowledgements:&BTreeSet<String>)->Result<Reviewed,String>{
  if !self.summary.ready_for_review||self.plan.is_none(){return Err("SOURCE_ADMISSION_REFUSED: resolve blocking source/native diagnostics first".into());}
  if acknowledgements!=&self.summary.required_acknowledgements{return Err("SOURCE_ADMISSION_EXACT_DISCLOSURE_REVIEW_REQUIRED".into());}
  Ok(Reviewed{source:self.source.clone(),summary:self.summary.clone(),plan:self.plan.as_ref().unwrap().clone()})
 }
}
impl Reviewed {
 pub fn summary(&self)->&Summary{&self.summary}
 /// Point-in-time original-list recheck, then one atomic native command. This
 /// does not lease external files or claim they remain current after the check.
 pub fn apply(self,e:&mut Editor)->Result<Applied,String>{
  let original_file_check=self.source.recheck()?;
  if !original_file_check.unchanged{return Err("SOURCE_ADMISSION_SOURCE_CHANGED: reviewed frozen source retained; prepare/review again explicitly".into());}
  self.plan.check(e)?;
  let checked_unix_milliseconds=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis();
  e.command(&e.freshness(),Command::Typed(types::Action::Admission(Box::new(self.plan))))?;
  Ok(Applied{summary:self.summary,original_file_check,checked_unix_milliseconds,target_model_revision:e.snapshot.model_revision,target_snapshot_sha256:e.snapshot.digest()})
 }
}
impl Summary {
 fn notice(&mut self,code:&str,detail:impl AsRef<str>,blocking:bool,node:Option<roxmltree::Node<'_,'_>>){
  if self.notices.len()>=MAX_NOTICES{self.omitted_notices+=1;return;}
  let range=node.map(|n|n.range());self.notices.push(Notice{code:code.into(),detail:detail.as_ref().chars().take(512).collect(),blocking,start:range.as_ref().map(|r|r.start),end:range.map(|r|r.end)});
 }
 fn blocked(&self)->bool{self.omitted_notices>0||self.notices.iter().any(|n|n.blocking)}
}
fn exact_id(s:&str)->bool{!s.is_empty()&&s.len()<=128&&s.bytes().enumerate().all(|(i,c)|c.is_ascii_alphabetic()||c==b'_'||(i>0&&(c.is_ascii_digit()||b"-.".contains(&c))))}
fn typed(n:roxmltree::Node<'_,'_>,local:&str)->bool{n.attribute((XMI,"type")).and_then(|v|v.split_once(':')).is_some_and(|(p,name)|name==local&&n.lookup_namespace_uri(Some(p))==Some(UML))}
fn plain(n:roxmltree::Node<'_,'_>,name:&str)->bool{n.tag_name().namespace().is_none()&&n.tag_name().name()==name}
fn span(n:roxmltree::Node<'_,'_>)->(usize,usize){(n.range().start,n.range().end)}
fn attrs(s:&mut Summary,n:roxmltree::Node<'_,'_>,allowed:&[(&str,&str)]){for a in n.attributes(){if !allowed.contains(&(a.namespace().unwrap_or(""),a.name())){s.notice("UNSUPPORTED_ATTRIBUTE",format!("{{{}}}{} is outside the editable subset; original bytes retained",a.namespace().unwrap_or(""),a.name()),true,Some(n));}}}
fn identity(s:&mut Summary,n:roxmltree::Node<'_,'_>)->String{let id=n.attribute((XMI,"id")).unwrap_or("");if !exact_id(id){s.notice("EXACT_ORIGINAL_ID_REQUIRED","Require an explicit 1..128-byte ASCII letter/underscore-led local ID using letters, digits, underscore, dot or hyphen",true,Some(n));}id.into()}
fn projection_key(s:&mut Summary,p:&v2::ProjectionSet,n:roxmltree::Node<'_,'_>,id:&str,local:&str)->Option<v2::ElementKey>{
 let key=p.nodes.iter().find(|x|x.location.start==n.range().start&&x.location.end==n.range().end&&x.key.identity==v2::ElementIdentity::Original(id.into())&&x.source_type.namespace==(if local=="Block"{native::BLOCK_NAMESPACE}else{UML})&&x.source_type.local==local).map(|x|x.key.clone());
 if key.is_none(){s.notice("EXISTING_PROJECTION_IDENTITY_REQUIRED","Exact original ID, source span and qualified type must match the existing semantic-set/2 projection",true,Some(n));}key
}
fn multiplicity(s:&mut Summary,n:roxmltree::Node<'_,'_>,covered:&mut BTreeSet<(usize,usize)>)->bool{
 let children=n.children().filter(|n|n.is_element()).collect::<Vec<_>>();
 let lower=n.attribute("lower");let upper=n.attribute("upper");
 if lower.is_some()||upper.is_some(){if lower!=Some("1")||upper!=Some("1")||!children.is_empty(){s.notice("EXPLICIT_ONE_TO_ONE_MULTIPLICITY_REQUIRED","Only exact explicit lower=1/upper=1, without duplicate child bounds, is represented",true,Some(n));return false;}return true;}
 let mut valid=children.len()==2;
 for (name,kind)in[("lowerValue","LiteralInteger"),("upperValue","LiteralUnlimitedNatural")]{let found=children.iter().filter(|c|plain(**c,name)).collect::<Vec<_>>();if found.len()!=1{valid=false;continue;}let child=**found.first().unwrap();covered.insert(span(child));attrs(s,child,&[(XMI,"id"),(XMI,"type"),("","value")]);if !typed(child,kind)||child.attribute("value")!=Some("1")||child.children().any(|n|n.is_element()){valid=false;}if let Some(id)=child.attribute((XMI,"id")){if !exact_id(id){valid=false;}}s.represented_multiplicity_literals+=1;}
 if !valid{s.notice("EXPLICIT_ONE_TO_ONE_MULTIPLICITY_REQUIRED","Require exact lowerValue LiteralInteger=1 and upperValue LiteralUnlimitedNatural=1; no default, range or extra child semantics inferred",true,Some(n));}valid
}
fn payload(s:&mut Summary,p:&v2::ProjectionSet,raw:&[u8])->Option<(native::Payload,BTreeMap<String,v2::ElementKey>)>{
 let text=match std::str::from_utf8(raw){Ok(t)=>t,Err(_)=>{s.notice("UTF8_XML_REQUIRED","Selected primary must be UTF-8 XML",true,None);return None;}};
 let doc=match roxmltree::Document::parse(text){Ok(d)=>d,Err(e)=>{s.notice("XML_REFUSED",e.to_string(),true,None);return None;}};
 let elements=doc.descendants().filter(|n|n.is_element()).collect::<Vec<_>>();
 if elements.len()>MAX_XML_ELEMENTS||elements.iter().any(|n|n.ancestors().count()>16){s.notice("XML_ELEMENT_OR_DEPTH_LIMIT","Maximum512 XML elements and16 ancestor levels for this first subset",true,None);return None;}
 if doc.descendants().any(|n|n.is_pi()){s.notice("PROCESSING_INSTRUCTION_UNSUPPORTED","Processing instructions are retained only in original bytes; no native admission",true,None);}
 if doc.descendants().any(|n|n.is_text()&&n.text().is_some_and(|t|!t.trim().is_empty())){s.notice("ELEMENT_TEXT_UNSUPPORTED","Non-whitespace element text is outside this attribute/literal component subset",true,None);}
 let comments=doc.descendants().filter(|n|n.is_comment()).count();if comments>0{s.notice("COMMENTS_RETAINED_IN_SOURCE_ONLY",format!("{comments} XML comments remain in original bytes; no native comment objects are created"),false,None);}
 let root=doc.root_element();let wrapper=root.tag_name().namespace()==Some(XMI)&&root.tag_name().name()=="XMI";let mut covered=BTreeSet::new();
 let models=if wrapper{covered.insert(span(root));attrs(s,root,&[(XMI,"version")]);if root.attribute((XMI,"version")).is_some_and(|v|!matches!(v,"2.5"|"2.5.1")){s.notice("XMI_VERSION_UNSUPPORTED","This wire subset accepts absent, 2.5 or 2.5.1 declared XMI version only",true,Some(root));}root.children().filter(|n|n.is_element()&&n.tag_name().namespace()==Some(UML)&&n.tag_name().name()=="Model").collect::<Vec<_>>()}else if root.tag_name().namespace()==Some(UML)&&root.tag_name().name()=="Model"{vec![root]}else{vec![]};
 if models.len()!=1{s.notice("ONE_EXPLICIT_UML_MODEL_REQUIRED","Require one direct UML20131001 Model, optionally inside XMI20131001 XMI",true,Some(root));return None;}
 let model=models[0];covered.insert(span(model));attrs(s,model,&[(XMI,"id"),("","name")]);let model_id=identity(s,model);let mut keys=BTreeMap::new();if let Some(key)=projection_key(s,p,model,&model_id,"Model"){keys.insert(model_id.clone(),key);}
 let mut payload=native::Payload{model_id,model_name:model.attribute("name").unwrap_or("").into(),components:vec![],usages:vec![]};let mut classes=BTreeMap::new();
 for n in model.children().filter(|n|n.is_element()){
  if !plain(n,"packagedElement")||!typed(n,"Class"){continue;}covered.insert(span(n));attrs(s,n,&[(XMI,"id"),(XMI,"type"),("","name"),("","isAbstract")]);let id=identity(s,n);
  if n.attribute("isAbstract")!=Some("false"){s.notice("EXPLICIT_CONCRETE_CLASS_REQUIRED","This first wire subset requires isAbstract=false explicitly; no abstract/default semantics inferred",true,Some(n));}
  if let Some(key)=projection_key(s,p,n,&id,"Class"){keys.insert(id.clone(),key);}classes.insert(id.clone(),n);payload.components.push(native::Component{source_id:id,name:n.attribute("name").unwrap_or("").into(),block:None});
 }
 for (owner,n)in &classes{for property in n.children().filter(|n|n.is_element()){
  if !plain(property,"ownedAttribute")||!typed(property,"Property"){continue;}covered.insert(span(property));attrs(s,property,&[(XMI,"id"),(XMI,"type"),("","name"),("","type"),("","aggregation"),("","lower"),("","upper")]);let id=identity(s,property);if let Some(key)=projection_key(s,p,property,&id,"Property"){keys.insert(id.clone(),key);}
  multiplicity(s,property,&mut covered);let target=property.attribute("type").unwrap_or("");
  if !classes.contains_key(target){s.notice("PAYLOAD_CLASS_TARGET_REQUIRED","Property type must be an exact local Class ID from this Model; no URI, native-ID or name inference",true,Some(property));}
  let kind=match property.attribute("aggregation"){Some("composite")=>types::UsageKind::Part,Some("none")=>types::UsageKind::Reference,_=>{s.notice("EXPLICIT_AGGREGATION_REQUIRED","Only explicit composite or none aggregation is represented; shared/default/unknown semantics refuse",true,Some(property));types::UsageKind::Reference}};
  let key=keys.get(&id);let resolved=key.is_some_and(|key|p.references.iter().any(|r|r.source==*key&&r.feature=="type"&&r.resolution==v2::ReferenceResolution::Resolved&&r.target.as_ref().is_some_and(|t|keys.get(target)==Some(t))));if !resolved{s.notice("EXISTING_RESOLVED_TYPE_REFERENCE_REQUIRED","The existing source projection must resolve the exact Property type to the selected local Class",true,Some(property));}
  payload.usages.push(native::ComponentUsage{source_id:id,name:property.attribute("name").unwrap_or("").into(),owner:owner.clone(),target:target.into(),kind});
 }}
 for block in elements.iter().copied().filter(|n|n.tag_name().namespace()==Some(native::BLOCK_NAMESPACE)&&n.tag_name().name()=="Block"){
  covered.insert(span(block));attrs(s,block,&[(XMI,"id"),("","base_Class")]);let id=identity(s,block);if let Some(key)=projection_key(s,p,block,&id,"Block"){keys.insert(id.clone(),key);}
  let parent_ok=wrapper&&block.parent_element()==Some(root);let base=block.attribute("base_Class").unwrap_or("");let target=payload.components.iter_mut().find(|c|c.source_id==base);
  if !parent_ok||target.as_ref().is_none_or(|c|c.block.is_some())||block.children().any(|n|n.is_element()){s.notice("BLOCK_DECLARATION_UNSUPPORTED","Require one empty root-sibling SysML20181001 Block declaration per local Class; no extra stereotype properties or semantics",true,Some(block));}else{target.unwrap().block=Some(native::BlockDeclaration{source_id:id,namespace:native::BLOCK_NAMESPACE.into()});}
 }
 for n in elements{if !covered.contains(&span(n)){s.notice("UNSUPPORTED_ELEMENT",format!("{{{}}}{} has no reviewed native representation; entire member refuses",n.tag_name().namespace().unwrap_or(""),n.tag_name().name()),true,Some(n));}}
 Some((payload,keys))
}
pub fn prepare(source:Arc<ProjectSet>,member:&str,e:&Editor,system:RecordId)->Prepared{
 let set=source.summary();let mut summary=Summary{rule:RULE.into(),source_set_digest:set.source_set_digest,selected_member:if member.len()<=64{member.into()}else{"[oversized member key refused]".into()},source_primary_sha256:None,system,target_native:e.freshness(),target_snapshot_sha256:e.snapshot.digest(),ready_for_review:false,required_acknowledgements:BTreeSet::new(),notices:vec![],omitted_notices:0,inspection_loss_count:0,inspection_diagnostic_count:0,inspection_details_omitted_budget:false,source_inspection_stage:None,source_inspection_losses:vec![],source_inspection_diagnostics:vec![],mappings:vec![],native_candidate:None,represented_multiplicity_literals:0,limits:vec!["Whole selected member only; strict XML shape/attribute coverage, no partial element selection or silent dropping. Other project-set members stay inspection-only.".into(),"Original primary and manifest bytes remain untouched. Recheck is point-in-time over the original file list, not an external file lease or durable currentness proof.".into(),"Existing semantic-set/2 inspection losses remain recorded as historical adapter output; its admission_preview remains false. This separate versioned adapter supports only the explicit reviewed subset.".into(),"Model container mapping, one-to-one usage representation and documentary Block/profile correspondence require explicit review. No full UML/SysML validity, namespace-version truth, execution, layout or engineering verification.".into(),"Limits:1MiBXML/512elements/16ancestorlevels/128notices/2MiBsummary, plus P38 and native64identity/128KiBpayload/type/profile/extension limits. No RSS/performance qualification.".into()]};
 let mut plan=None;
 if let Some(selected)=set.members.iter().find(|m|m.key==member){
  summary.source_primary_sha256=Some(selected.primary.expected_sha256.clone());
  if selected.format!=Format::SemanticV2||selected.primary.state!=FileState::Verified||!selected.companions.is_empty(){summary.notice("EXACT_SINGLE_SEMANTIC_PRIMARY_REQUIRED","Require verified SemanticV2 primary with no companion semantics; native/changed/missing/companion-bearing members remain inspection-only",true,None);}
  if let(Some(p),Some(raw))=(source.semantic(member),source.primary_bytes(member)){
   summary.inspection_loss_count=p.losses.len();summary.inspection_diagnostic_count=p.diagnostics.len();summary.source_inspection_stage=Some(p.stage.clone());summary.source_inspection_losses=p.losses.clone();summary.source_inspection_diagnostics=p.diagnostics.clone();
   if p.stage!=v2::ProjectionStage::PartialSemanticProjection||p.assets.len()!=1||p.counters.omitted_diagnostics>0||p.counters.omitted_losses>0||raw.len()>MAX_XML_BYTES||raw.starts_with(b"PK"){summary.notice("BOUNDED_COMPLETE_XML_INSPECTION_REQUIRED","Require one bounded unarchived primary and untruncated existing XML projection; unsupported/refused/archive stages remain inspection-only",true,None);}
   if !summary.blocked(){if let Some((payload,keys))=payload(&mut summary,p,raw){if !summary.blocked(){match native::prepare(e,system,native::Source{member:member.into(),primary_sha256:selected.primary.expected_sha256.clone()},payload){
    Ok(candidate)=>{for c in &candidate.summary().correspondences{if let Some(key)=keys.get(&c.source_id){summary.mappings.push(Mapping{source:ElementKey{member:member.into(),primary_sha256:selected.primary.expected_sha256.clone(),identity:LocalIdentity::External(key.clone())},native:c.native,native_kind:c.native_kind,source_kind:c.source_kind.clone()});}else{summary.notice("CORRESPONDENCE_COVERAGE_REQUIRED","Every native correspondence must retain its exact existing source key",true,None);}}summary.native_candidate=Some(candidate.summary().clone());plan=Some(candidate);},
    Err(error)=>summary.notice("NATIVE_PAYLOAD_REFUSED",error,true,None),
   }}}}
  }else{summary.notice("SELECTED_SOURCE_INTERPRETATION_UNAVAILABLE","Prior source bytes and diagnostics are retained; no editable candidate",true,None);}
 }else{summary.notice("EXACT_MEMBER_REQUIRED","Choose one exact member key from this frozen project set",true,None);}
 if plan.is_some(){summary.notice("MODEL_CONTAINER_MAPPING","Source Model maps to the explicitly selected existing System without renaming it; source model ID/name remain documentary correspondence metadata",false,None);summary.notice("ONE_TO_ONE_USAGE_MAPPING","Supported Class properties are represented as existing one-to-one Part/Reference usages; multiplicity literal helper IDs remain in original source bytes, not separate native objects",false,None);if summary.native_candidate.as_ref().is_some_and(|n|n.declared_blocks>0){summary.notice("BLOCK_DECLARATION_ONLY","Block source declarations become documentary native profile tags on component definitions; full SysML metamodel semantics are not claimed",false,None);}if set.members.len()>1{summary.notice("OTHER_MEMBERS_NOT_IMPORTED",format!("Only selected member {member} is considered; {} other members stay inspection-only",set.members.len()-1),false,None);}}
 if serde_json::to_vec(&summary).map_or(true,|b|b.len()>MAX_SUMMARY_BYTES){summary.source_inspection_losses.clear();summary.source_inspection_diagnostics.clear();summary.inspection_details_omitted_budget=true;summary.notice("SUMMARY_OUTPUT_BUDGET_REFUSED","Complete review cannot fit the output budget; no partial candidate is admitted",true,None);}
 summary.ready_for_review=plan.is_some()&&!summary.blocked();if !summary.ready_for_review{plan=None;summary.mappings.clear();summary.native_candidate=None;}else{summary.required_acknowledgements=summary.notices.iter().filter(|n|!n.blocking).map(|n|n.code.clone()).collect();}
 Prepared{source,summary,plan}
}
