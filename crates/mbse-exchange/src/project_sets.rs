//! Explicit hash-pinned local project members. Frozen inspection only; no target writes or fetch.
use crate::v2;
use mbse_model::{local_input,native_views::{self as views,Document},stage1::{Snapshot,Record,RecordId},stage1_io};
use serde::{Serialize,Deserialize};
use sha2::{Digest,Sha256};
use std::{collections::BTreeSet,io::Write,path::{Path,PathBuf,Component}};
pub const PROFILE:&str="mbse-craft.source-project-set/1";
pub const MAX_MANIFEST_BYTES:usize=128*1024;
pub const MAX_MEMBERS:usize=16;
pub const MAX_ARTIFACTS:usize=64;
pub const MAX_CAPTURE_BYTES:usize=64*1024*1024;
pub const MAX_EXPANDED_BYTES:usize=64*1024*1024;
pub const MAX_INTERPRETATION_BYTES:usize=64*1024*1024;
pub const MAX_ELEMENTS:usize=100_000;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Format {NativeSaved,SemanticV2}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum CompanionRole {NativeViews,Opaque}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpec {pub path:String,pub sha256:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanionSpec {pub role:CompanionRole,pub file:FileSpec}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberSpec {pub key:String,pub format:Format,pub primary:FileSpec,#[serde(default)]pub companions:Vec<CompanionSpec>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {pub profile:String,pub schema_version:u32,pub members:Vec<MemberSpec>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub enum FileState {Verified,Changed,Missing,Unavailable(String)}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct FileInfo {pub path:String,pub expected_sha256:String,pub actual_sha256:Option<String>,pub bytes:Option<usize>,pub state:FileState}
struct FrozenFile {info:FileInfo,bytes:Option<Vec<u8>>}
struct Companion {spec:CompanionSpec,file:FrozenFile,views:Option<Document>,issue:Option<String>}
enum Data {Native(Snapshot),Semantic(v2::ProjectionSet)}
struct Member {spec:MemberSpec,primary:FrozenFile,companions:Vec<Companion>,data:Option<Data>,issue:Option<String>}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Serialize)]
pub enum LocalIdentity {NativeRecord(RecordId),External(v2::ElementKey)}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Serialize)]
pub struct ElementKey {pub member:String,pub primary_sha256:String,pub identity:LocalIdentity}
#[derive(Clone,Debug,Serialize)]
pub struct CompanionSummary {pub role:CompanionRole,pub file:FileInfo,pub bound_declaration_checked:bool,pub issue:Option<String>}
#[derive(Clone,Debug,Serialize)]
pub struct MemberSummary {pub key:String,pub format:Format,pub primary:FileInfo,pub companions:Vec<CompanionSummary>,pub native_document:Option<String>,pub native_top_level_records:Option<usize>,pub external_stage:Option<v2::ProjectionStage>,pub external_nodes:Option<usize>,pub external_losses:Option<usize>,pub issue:Option<String>}
#[derive(Clone,Debug,Serialize)]
pub struct Summary {pub profile:String,pub manifest_path:PathBuf,pub manifest_sha256:String,pub source_set_digest:String,pub captured_bytes:usize,pub expanded_bytes:usize,pub interpretation_bytes:usize,pub members:Vec<MemberSummary>,pub limits:Vec<String>}
#[derive(Clone,Debug,Serialize)]
pub struct Recheck {pub unchanged:bool,pub manifest:FileInfo,pub files:Vec<(String,FileInfo)>}
/// Frozen source data, never a live editor/checkpoint or native admission authority.
pub struct ProjectSet {manifest_path:PathBuf,root:PathBuf,manifest_bytes:Vec<u8>,manifest:Manifest,members:Vec<Member>,source_set_digest:String,captured_bytes:usize,expanded_bytes:usize,interpretation_bytes:usize}
fn hash(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn detail(s:impl AsRef<str>)->String{s.as_ref().chars().take(512).collect()}
fn sha(value:&str)->bool{value.len()==64&&value.bytes().all(|c|c.is_ascii_digit()||(b'a'..=b'f').contains(&c))}
fn relative(value:&str)->bool{!value.is_empty()&&value.len()<=512&&!value.starts_with('/')&&!value.contains(['\\',':','\0'])&&!value.chars().any(char::is_control)&&value.split('/').all(|p|!p.is_empty()&&p!="."&&p!="..")}
fn validate(m:&Manifest)->Result<(),String>{
 if m.profile!=PROFILE||m.schema_version!=1{return Err("PROJECT_SET_PROFILE_OR_VERSION".into());}if m.members.is_empty()||m.members.len()>MAX_MEMBERS{return Err("PROJECT_SET_MEMBER_LIMIT".into());}
 let mut keys=BTreeSet::new();let mut count=0;
 for member in &m.members{
  if member.key.is_empty()||member.key.len()>64||!member.key.bytes().all(|c|c.is_ascii_alphanumeric()||b"_-".contains(&c))||!keys.insert(member.key.clone()){return Err("PROJECT_SET_EXACT_UNIQUE_MEMBER_KEY".into());}
  let mut paths=BTreeSet::new();
  for file in std::iter::once(&member.primary).chain(member.companions.iter().map(|c|&c.file)){count+=1;if !relative(&file.path)||!sha(&file.sha256)||!paths.insert(file.path.clone()){return Err("PROJECT_SET_RELATIVE_PATH_HASH_OR_DUPLICATE".into());}}
  if member.format!=Format::NativeSaved&&member.companions.iter().any(|c|c.role==CompanionRole::NativeViews){return Err("PROJECT_SET_NATIVE_VIEWS_REQUIRE_NATIVE_MEMBER".into());}
 }
 if count>MAX_ARTIFACTS{return Err("PROJECT_SET_ARTIFACT_LIMIT".into());}Ok(())
}
fn absolute(path:&Path)->Result<PathBuf,String>{
 let path=if path.is_absolute(){path.to_owned()}else{std::env::current_dir().map_err(|e|e.to_string())?.join(path)};
 if path.components().any(|c|matches!(c,Component::ParentDir|Component::CurDir)){return Err("PROJECT_SET_EXACT_LOCAL_PATH_REQUIRED".into());}local_input::validate_path(&path,"PROJECT_SET")?;Ok(path)
}
fn capture(directory:&local_input::rooted::Directory,file:&FileSpec,limit:usize,used:&mut usize)->Result<FrozenFile,String>{
 let mut info=FileInfo{path:file.path.clone(),expected_sha256:file.sha256.clone(),actual_sha256:None,bytes:None,state:FileState::Missing};
 let bytes=match directory.read(&file.path,limit.min(MAX_CAPTURE_BYTES.saturating_sub(*used)),"PROJECT_SET_FILE"){Ok(bytes)=>bytes,Err(e)if e.contains("FILE_LIMIT")=>return Err("PROJECT_SET_INPUT_BYTE_BUDGET".into()),Err(e)if e.contains("FILE_MISSING")=>return Ok(FrozenFile{info,bytes:None}),Err(e)=>{info.state=FileState::Unavailable(detail(e));return Ok(FrozenFile{info,bytes:None});}};
 *used=used.checked_add(bytes.len()).filter(|n|*n<=MAX_CAPTURE_BYTES).ok_or("PROJECT_SET_INPUT_BYTE_BUDGET")?;let actual=hash(&bytes);info.state=if actual==file.sha256{FileState::Verified}else{FileState::Changed};info.actual_sha256=Some(actual);info.bytes=Some(bytes.len());Ok(FrozenFile{info,bytes:Some(bytes)})
}
struct Count {used:usize,limit:usize}
impl Write for Count{fn write(&mut self,b:&[u8])->std::io::Result<usize>{self.used=self.used.checked_add(b.len()).filter(|n|*n<=self.limit).ok_or_else(||std::io::Error::other("PROJECT_SET_INTERPRETATION_BUDGET"))?;Ok(b.len())}fn flush(&mut self)->std::io::Result<()>{Ok(())}}
fn encoded_size(value:&impl Serialize,limit:usize)->Result<usize,String>{let mut count=Count{used:0,limit};serde_json::to_writer(&mut count,value).map_err(|_|"PROJECT_SET_INTERPRETATION_BUDGET".to_string())?;Ok(count.used)}
fn primary_limit(format:&Format)->usize{match format{Format::NativeSaved=>stage1_io::MAX_BYTES,Format::SemanticV2=>v2::MAX_PRIMARY}}
fn companion_limit(role:&CompanionRole)->usize{match role{CompanionRole::NativeViews=>views::MAX_BYTES,CompanionRole::Opaque=>v2::MAX_ADDITIONAL}}
impl ProjectSet {
 pub fn open(path:&Path)->Result<Self,String>{
  let path=absolute(path)?;let root=path.parent().ok_or("PROJECT_SET_ROOT")?.to_owned();let directory=local_input::rooted::Directory::open(&root)?;let name=path.file_name().and_then(|s|s.to_str()).ok_or("PROJECT_SET_MANIFEST_NAME")?;let bytes=directory.read(name,MAX_MANIFEST_BYTES,"PROJECT_SET_MANIFEST")?;let manifest:Manifest=serde_json::from_slice(&bytes).map_err(|e|format!("PROJECT_SET_MANIFEST_SCHEMA: {e}"))?;validate(&manifest)?;let mut used=bytes.len();let mut expanded=0usize;let mut interpreted=0usize;let mut elements=0usize;let mut members=Vec::new();
  for spec in &manifest.members{
   let primary=capture(&directory,&spec.primary,primary_limit(&spec.format),&mut used)?;let mut issue=None;let mut data=None;
   if primary.info.state==FileState::Verified{
    let raw=primary.bytes.as_ref().expect("captured verified bytes");
    match spec.format{
     Format::NativeSaved=>match stage1_io::inspect_saved_bytes(raw){Ok(snapshot)=>{let size=encoded_size(&snapshot,MAX_INTERPRETATION_BYTES.saturating_sub(interpreted))?;interpreted+=size;elements+=snapshot.records.len();data=Some(Data::Native(snapshot));},Err(e)=>issue=Some(detail(e))},
     Format::SemanticV2=>match v2::inspect_primary_bytes(&spec.primary.path,raw.clone()){Ok(report)=>{let additional=report.assets.iter().filter(|a|a.role!=v2::AssetRole::Primary).map(|a|a.byte_count).sum::<usize>();expanded=expanded.checked_add(additional).filter(|n|*n<=MAX_EXPANDED_BYTES).ok_or("PROJECT_SET_EXPANDED_BYTE_BUDGET")?;interpreted=interpreted.checked_add(report.counters.serialized_report_bytes).filter(|n|*n<=MAX_INTERPRETATION_BYTES).ok_or("PROJECT_SET_INTERPRETATION_BUDGET")?;elements+=report.nodes.len();data=Some(Data::Semantic(report));},Err(e)=>issue=Some(detail(e))}
    }
   }else{issue=Some("Primary bytes do not match the explicit pin; no interpretation performed.".into());}
   if elements>MAX_ELEMENTS{return Err("PROJECT_SET_ELEMENT_BUDGET".into());}
   let mut companions=Vec::new();for c in &spec.companions{let file=capture(&directory,&c.file,companion_limit(&c.role),&mut used)?;let mut declared=None;let mut companion_issue=None;
    if c.role==CompanionRole::NativeViews{if file.info.state==FileState::Verified{if let Some(Data::Native(snapshot))=&data{match views::inspect_bound_declaration(file.bytes.as_ref().unwrap(),snapshot,&spec.primary.sha256){Ok(document)=>{interpreted+=encoded_size(&document,MAX_INTERPRETATION_BYTES.saturating_sub(interpreted))?;declared=Some(document);},Err(e)=>companion_issue=Some(detail(e))}}else{companion_issue=Some("Native primary unavailable; companion binding not checked.".into());}}else{companion_issue=Some("Companion missing, unavailable or changed; binding not checked.".into());}}
    companions.push(Companion{spec:c.clone(),file,views:declared,issue:companion_issue});
   }
   members.push(Member{spec:spec.clone(),primary,companions,data,issue});
  }
  let mut set=Self{manifest_path:path,root,manifest_bytes:bytes,manifest,members,source_set_digest:String::new(),captured_bytes:used,expanded_bytes:expanded,interpretation_bytes:interpreted};let summary=set.summary();set.source_set_digest=hash(&serde_json::to_vec(&(PROFILE,summary.manifest_sha256,summary.members.iter().map(|m|(&m.key,&m.format,&m.primary,&m.companions)).collect::<Vec<_>>())).map_err(|e|e.to_string())?);Ok(set)
 }
 pub fn summary(&self)->Summary{
  let members=self.members.iter().map(|m|{let(native_document,native_top_level_records,external_stage,external_nodes,external_losses)=match &m.data{Some(Data::Native(s))=>(Some(s.document_id.0.clone()),Some(s.records.len()),None,None,None),Some(Data::Semantic(s))=>(None,None,Some(s.stage.clone()),Some(s.nodes.len()),Some(s.losses.len())),None=>(None,None,None,None,None)};MemberSummary{key:m.spec.key.clone(),format:m.spec.format.clone(),primary:m.primary.info.clone(),companions:m.companions.iter().map(|c|CompanionSummary{role:c.spec.role.clone(),file:c.file.info.clone(),bound_declaration_checked:c.views.is_some(),issue:c.issue.clone()}).collect(),native_document,native_top_level_records,external_stage,external_nodes,external_losses,issue:m.issue.clone()}}).collect();
  Summary{profile:PROFILE.into(),manifest_path:self.manifest_path.clone(),manifest_sha256:hash(&self.manifest_bytes),source_set_digest:self.source_set_digest.clone(),captured_bytes:self.captured_bytes,expanded_bytes:self.expanded_bytes,interpretation_bytes:self.interpretation_bytes,members,limits:vec!["Frozen explicit local files only. No directory discovery, network, ambient URI/companion fetch, calculation, target mutation or native admission. Directory-relative NOFOLLOW reads are currently qualified on Unix; other platforms explicitly refuse this route.".into(),"Member key plus exact primary hash, source kind and local identity keep equal local IDs distinct even for identical copied projects. Reopened changed pins do not silently retarget old keys.".into(),"Native snapshots use the existing saved-native decoder; bound presentations validate the existing schemas against exact supplied native bytes. Read-only DTOs cannot mint native checkpoints, runtime inverse proofs or write/admission authority.".into(),"External primary inspection uses the existing semantic-set/2 adapter; its partial/refused stages, losses and unresolved resources remain explicit. Declared opaque companions are byte-retained only; no cross-project semantic/reference merge is inferred.".into(),"File Verified means captured bytes match an explicit pin, not source truth, semantic completeness, currentness or engineering acceptance. Missing/changed/unavailable files remain visible; prior frozen data is retained during recheck.".into(),"Limits:16members/64artifacts/128KiBmanifest/64MiBcaptured/64MiBadditionalarchivebytes/64MiBinterpretedserialization/100000elements. Encoded budgets are not RSS or performance guarantees.".into()]}
 }
 pub fn native_snapshot(&self,key:&str)->Option<&Snapshot>{self.members.iter().find(|m|m.spec.key==key).and_then(|m|if let Some(Data::Native(s))=&m.data{Some(s)}else{None})}
 pub fn primary_bytes(&self,key:&str)->Option<&[u8]>{self.members.iter().find(|m|m.spec.key==key)?.primary.bytes.as_deref()}
 pub fn companion_bytes(&self,key:&str,index:usize)->Option<&[u8]>{self.members.iter().find(|m|m.spec.key==key)?.companions.get(index)?.file.bytes.as_deref()}
 pub fn bound_views(&self,key:&str,index:usize)->Option<&Document>{self.members.iter().find(|m|m.spec.key==key)?.companions.get(index)?.views.as_ref()}
 pub fn semantic(&self,key:&str)->Option<&v2::ProjectionSet>{self.members.iter().find(|m|m.spec.key==key).and_then(|m|if let Some(Data::Semantic(s))=&m.data{Some(s)}else{None})}
 pub fn keys(&self,key:&str)->Vec<ElementKey>{let Some(member)=self.members.iter().find(|m|m.spec.key==key)else{return Vec::new();};let identities:Vec<LocalIdentity>=match &member.data{Some(Data::Native(s))=>s.records.keys().copied().map(LocalIdentity::NativeRecord).collect(),Some(Data::Semantic(s))=>s.nodes.iter().map(|n|LocalIdentity::External(n.key.clone())).collect(),None=>Vec::new()};identities.into_iter().map(|identity|ElementKey{member:key.into(),primary_sha256:member.spec.primary.sha256.clone(),identity}).collect()}
 pub fn native_record(&self,key:&ElementKey)->Option<&Record>{let m=self.members.iter().find(|m|m.spec.key==key.member&&m.spec.primary.sha256==key.primary_sha256)?;if let(Some(Data::Native(s)),LocalIdentity::NativeRecord(id))=(&m.data,&key.identity){s.records.get(id)}else{None}}
 pub fn external_node(&self,key:&ElementKey)->Option<&v2::SemanticNode>{let m=self.members.iter().find(|m|m.spec.key==key.member&&m.spec.primary.sha256==key.primary_sha256)?;if let(Some(Data::Semantic(s)),LocalIdentity::External(id))=(&m.data,&key.identity){s.nodes.iter().find(|n|n.key==*id)}else{None}}
 /// Re-read only the original descriptor and originally listed files. Never
 /// parse a changed descriptor or follow any newly proposed resource location.
 pub fn recheck(&self)->Result<Recheck,String>{
  let file=FileSpec{path:self.manifest_path.file_name().and_then(|s|s.to_str()).ok_or("PROJECT_SET_MANIFEST_NAME")?.into(),sha256:hash(&self.manifest_bytes)};let directory=local_input::rooted::Directory::open(&self.root)?;let mut used=0;let observed=capture(&directory,&file,MAX_MANIFEST_BYTES,&mut used)?;let mut unchanged=observed.info.state==FileState::Verified;let mut files=Vec::new();
  for member in &self.members{let primary=capture(&directory,&member.spec.primary,primary_limit(&member.spec.format),&mut used)?;unchanged&=primary.info==member.primary.info;files.push((member.spec.key.clone(),primary.info));for companion in &member.companions{let file=capture(&directory,&companion.spec.file,companion_limit(&companion.spec.role),&mut used)?;unchanged&=file.info==companion.file.info;files.push((member.spec.key.clone(),file.info));}}
  Ok(Recheck{unchanged,manifest:observed.info,files})
 }
}
