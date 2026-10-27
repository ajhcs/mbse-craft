//! Source-only declarations. Does not inspect, prepare, review or apply any model.
use std::path::{Path,PathBuf};
use mbse_exchange::project_sets::{Manifest,MemberSpec,FileSpec,Format,PROFILE};
use sha2::{Digest,Sha256};
pub const XML:&str=r##"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:uml="http://www.omg.org/spec/UML/20131001" xmlns:sysml="http://www.omg.org/spec/SysML/20181001/SysML" xmi:version="2.5.1">
<!-- Synthetic source fixture; not a vendor-created project or engineering acceptance. -->
<uml:Model xmi:id="model" name="Synthetic source model">
<packagedElement xmi:type="uml:Class" xmi:id="A" name="Imported Assembly" isAbstract="false">
<ownedAttribute xmi:type="uml:Property" xmi:id="part" name="sensor" type="B" aggregation="composite"><lowerValue xmi:type="uml:LiteralInteger" xmi:id="lowerP" value="1"/><upperValue xmi:type="uml:LiteralUnlimitedNatural" xmi:id="upperP" value="1"/></ownedAttribute>
</packagedElement>
<packagedElement xmi:type="uml:Class" xmi:id="B" name="Imported Sensor" isAbstract="false"><ownedAttribute xmi:type="uml:Property" xmi:id="ref" name="assemblyReference" type="A" aggregation="none" lower="1" upper="1"/></packagedElement>
</uml:Model>
<sysml:Block xmi:id="blockA" base_Class="A"/>
</xmi:XMI>"##;
pub fn prepare(root:&Path,xml:&[u8])->PathBuf{
 let primary=root.join("source.xmi");assert!(!primary.exists());std::fs::write(&primary,xml).unwrap();let file=FileSpec{path:"source.xmi".into(),sha256:format!("{:x}",Sha256::digest(xml))};
 let manifest=Manifest{profile:PROFILE.into(),schema_version:1,members:["supplier","consumer"].into_iter().map(|key|MemberSpec{key:key.into(),format:Format::SemanticV2,primary:file.clone(),companions:vec![]}).collect()};let path=root.join("source.mbse-projects.json");assert!(!path.exists());std::fs::write(&path,serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();path
}
