//! Source-only fixture preparation; never opens or interprets a project set.
use std::path::{Path,PathBuf};
use mbse_exchange::project_sets::{Manifest,MemberSpec,FileSpec,Format,CompanionSpec,CompanionRole,PROFILE};
use sha2::{Digest,Sha256};
pub const NATIVE:&[u8]=include_bytes!("../../../../docs/parity/p37b/verification/native-02/run/metrics-final-cold-copy.kernspan");
pub const VIEWS:&[u8]=include_bytes!("../../../../docs/parity/p37b/verification/native-02/run/metrics-final-cold-copy.mbse-views.json");
pub fn prepare(root:&Path)->PathBuf{
 fn file(root:&Path,name:&str,bytes:&[u8])->FileSpec{let path=root.join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();assert!(!path.exists());std::fs::write(path,bytes).unwrap();FileSpec{path:name.into(),sha256:format!("{:x}",Sha256::digest(bytes))}}
 let native=file(root,"left/native.kernspan",NATIVE);let views=file(root,"left/native.mbse-views.json",VIEWS);
 let mut right:serde_json::Value=serde_json::from_slice(NATIVE).unwrap();right["snapshot"]["records"]["1"]["name"]=serde_json::json!("Supplier System with the same native document and local ID");let right=file(root,"right/native.kernspan",&serde_json::to_vec_pretty(&right).unwrap());
 let mut changed=file(root,"left/changed.txt",b"Actual retained companion bytes");changed.sha256=format!("{:x}",Sha256::digest(b"Expected previous companion bytes"));
 let xml=br##"<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:uml="http://www.omg.org/spec/UML/20131001"><uml:Model xmi:id="model" name="Synthetic offline model"><packagedElement xmi:type="uml:Class" xmi:id="dup" name="Shared local ID"/><packagedElement xmi:type="uml:Class" href="https://invalid.example/library.uml#remote"/></uml:Model></xmi:XMI>"##;
 let external=file(root,"external/source.xmi",xml);
 let manifest=Manifest{profile:PROFILE.into(),schema_version:1,members:vec![
  MemberSpec{key:"left".into(),format:Format::NativeSaved,primary:native,companions:vec![CompanionSpec{role:CompanionRole::NativeViews,file:views},CompanionSpec{role:CompanionRole::Opaque,file:FileSpec{path:"left/missing.txt".into(),sha256:"0".repeat(64)}},CompanionSpec{role:CompanionRole::Opaque,file:changed}]},
  MemberSpec{key:"right".into(),format:Format::NativeSaved,primary:right,companions:vec![]},
  MemberSpec{key:"supplier".into(),format:Format::SemanticV2,primary:external.clone(),companions:vec![]},
  MemberSpec{key:"consumer".into(),format:Format::SemanticV2,primary:external,companions:vec![]},
 ]};let path=root.join("set.mbse-projects.json");assert!(!path.exists());std::fs::write(&path,serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();path
}
