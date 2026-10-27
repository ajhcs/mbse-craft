use mbse_exchange::{source_admission as a,project_sets::{self as sets,ProjectSet},v2};
use mbse_model::{stage1::*,types::{self as t}};
use std::{path::PathBuf,sync::Arc,collections::BTreeSet};
use serde_json::{Value,json};
#[path="support/admission_source.rs"]mod fixture;
const NATIVE:&[u8]=include_bytes!("../../../docs/parity/p38b/verification/native-01/cold-run/project-set-target-cold-copy.kernspan");
fn editor()->Editor{let value:Value=serde_json::from_slice(NATIVE).unwrap();Editor::new(serde_json::from_value(value["snapshot"].clone()).unwrap(),"source-admission:runtime".into()).unwrap()}
fn source(xml:&str)->(tempfile::TempDir,PathBuf,Arc<ProjectSet>){let root=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let path=fixture::prepare(root.path(),xml.as_bytes());let set=Arc::new(ProjectSet::open(&path).unwrap());(root,path,set)}
fn prepared(xml:&str)->(tempfile::TempDir,PathBuf,Editor,a::Prepared){let(root,path,set)=source(xml);let e=editor();let p=a::prepare(set,"supplier",&e,RecordId(1));(root,path,e,p)}
fn ready(p:&a::Prepared){assert!(p.summary().ready_for_review,"{}",serde_json::to_string_pretty(p.summary()).unwrap());}
fn refused(xml:&str,code:&str){let(root,path,e,p)=prepared(xml);assert!(!p.summary().ready_for_review);assert!(p.summary().notices.iter().any(|n|n.code==code),"{}",serde_json::to_string_pretty(p.summary()).unwrap());assert!(p.review(&BTreeSet::new()).is_err());assert_eq!(std::fs::read(root.path().join("source.xmi")).unwrap(),xml.as_bytes());assert!(path.exists());assert_eq!(e.snapshot,editor().snapshot);}
#[test]
fn source_admission_strict_sysml_component_subset_reuses_existing_projection_keys_and_never_changes_prepared_source_or_target(){
 let(root,path,e,p)=prepared(fixture::XML);ready(&p);let s=p.summary();assert_eq!(s.mappings.len(),6);assert_eq!(s.native_candidate.as_ref().unwrap().created_components,2);assert_eq!(s.native_candidate.as_ref().unwrap().created_usages,2);assert_eq!(s.native_candidate.as_ref().unwrap().declared_blocks,1);assert_eq!(s.represented_multiplicity_literals,2);assert_eq!(s.target_snapshot_sha256,e.snapshot.digest());assert!(p.target_current(&e));assert_eq!(s.source_inspection_stage,Some(v2::ProjectionStage::PartialSemanticProjection));assert_eq!(s.inspection_loss_count,s.source_inspection_losses.len());assert_eq!(s.inspection_omitted_losses,0);assert_eq!(s.inspection_omitted_diagnostics,0);assert!(s.required_acknowledgements.contains("BLOCK_DECLARATION_ONLY"));assert!(s.required_acknowledgements.contains("COMMENTS_RETAINED_IN_SOURCE_ONLY"));let set=ProjectSet::open(&path).unwrap();assert!(s.mappings.iter().all(|m|set.external_node(&m.source).is_some()));assert!(!set.semantic("supplier").unwrap().admission_preview(&[]).admitted);assert_eq!(std::fs::read(root.path().join("source.xmi")).unwrap(),fixture::XML.as_bytes());assert_eq!(e.snapshot,editor().snapshot);
}
#[test]
fn source_admission_plain_uml_and_standalone_model_are_explicit_without_inferring_unused_block_namespace_claims(){
 let plain=fixture::XML.replace("<sysml:Block xmi:id=\"blockA\" base_Class=\"A\"/>","");let(_r,_path,_e,p)=prepared(&plain);ready(&p);assert_eq!(p.summary().native_candidate.as_ref().unwrap().declared_blocks,0);assert!(!p.summary().required_acknowledgements.contains("BLOCK_DECLARATION_ONLY"));let standalone=r##"<uml:Model xmlns:uml="http://www.omg.org/spec/UML/20131001" xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmi:id="m" name="M"><packagedElement xmi:type="uml:Class" xmi:id="C" name="C" isAbstract="false"/></uml:Model>"##;let(_r,_path,_e,p)=prepared(standalone);ready(&p);assert_eq!(p.summary().mappings.len(),2);
}
#[test]
fn source_admission_exact_complete_disclosure_review_is_required_without_losing_the_prepared_candidate(){
 let(_r,_path,e,p)=prepared(fixture::XML);ready(&p);let required=p.summary().required_acknowledgements.clone();assert!(p.review(&BTreeSet::new()).is_err());let mut missing=required.clone();missing.pop_first();assert!(p.review(&missing).is_err());let mut extra=required.clone();extra.insert("UNREVIEWED_CODE".into());assert!(p.review(&extra).is_err());assert!(p.target_current(&e));let reviewed=p.review(&required).unwrap();assert_eq!(reviewed.summary().source_set_digest,p.summary().source_set_digest);assert_eq!(e.snapshot,editor().snapshot);
}
#[test]
fn source_admission_unsupported_attributes_and_namespace_rebindings_refuse_entire_member_with_located_losses(){
 for xml in [fixture::XML.replacen("isAbstract=\"false\"","isAbstract=\"false\" isLeaf=\"true\"",1),fixture::XML.replacen("name=\"Synthetic source model\"","name=\"Synthetic source model\" visibility=\"public\"",1)]{refused(&xml,"UNSUPPORTED_ATTRIBUTE");let(_r,_path,_e,p)=prepared(&xml);assert!(p.summary().notices.iter().any(|n|n.code=="UNSUPPORTED_ATTRIBUTE"&&n.start.is_some()&&n.end.is_some()));}
 let xml=fixture::XML.replace("xmlns:uml=\"http://www.omg.org/spec/UML/20131001\"","xmlns:uml=\"urn:unknown:UML\"");let(_r,_path,_e,p)=prepared(&xml);assert!(!p.summary().ready_for_review);
}
#[test]
fn source_admission_untyped_unidentified_children_operations_generalizations_extensions_and_nested_packages_cannot_disappear(){
 for extra in ["<generalization general=\"B\"/>","<ownedOperation name=\"doWork\"/>","<unknownSemanticThing/>","<packagedElement xmi:type=\"uml:Package\" xmi:id=\"nested\" name=\"Nested\"/>","<xmi:Extension extender=\"Unsupported vendor semantics\"/>"]{let xml=fixture::XML.replacen("</packagedElement>",&format!("{extra}</packagedElement>"),1);refused(&xml,"UNSUPPORTED_ELEMENT");}
}
#[test]
fn source_admission_abstract_default_class_other_classifier_kinds_and_unknown_versions_refuse_instead_of_reclassification(){
 for xml in [fixture::XML.replacen(" isAbstract=\"false\"","",1),fixture::XML.replacen("isAbstract=\"false\"","isAbstract=\"true\"",1)]{refused(&xml,"EXPLICIT_CONCRETE_CLASS_REQUIRED");}for kind in ["DataType","PrimitiveType","Interface","Activity"]{let xml=fixture::XML.replacen("xmi:type=\"uml:Class\"",&format!("xmi:type=\"uml:{kind}\""),1);refused(&xml,"UNSUPPORTED_ELEMENT");}refused(&fixture::XML.replace("xmi:version=\"2.5.1\"","xmi:version=\"99\""),"XMI_VERSION_UNSUPPORTED");
}
#[test]
fn source_admission_only_explicit_exact_one_to_one_property_bounds_are_represented_without_default_or_collection_inference(){
 for xml in [fixture::XML.replacen("value=\"1\"","value=\"0\"",1),fixture::XML.replacen("upper=\"1\"","upper=\"*\"",1),fixture::XML.replace(" lower=\"1\" upper=\"1\"",""),fixture::XML.replacen("aggregation=\"composite\"","aggregation=\"composite\" lower=\"1\" upper=\"1\"",1)]{refused(&xml,"EXPLICIT_ONE_TO_ONE_MULTIPLICITY_REQUIRED");}for xml in [fixture::XML.replace("aggregation=\"none\"","aggregation=\"shared\""),fixture::XML.replace(" aggregation=\"none\"","")]{refused(&xml,"EXPLICIT_AGGREGATION_REQUIRED");}
}
#[test]
fn source_admission_uri_unknown_native_id_and_name_type_references_refuse_without_ambient_or_cross_member_resolution(){
 for target in ["https://invalid.example/library.uml#B","missing","1009","Imported Sensor","consumer#B"]{let xml=fixture::XML.replacen("type=\"B\"",&format!("type=\"{target}\""),1);refused(&xml,"PAYLOAD_CLASS_TARGET_REQUIRED");}
}
#[test]
fn source_admission_block_claim_requires_exact_namespace_single_local_base_and_no_extra_stereotype_properties(){
 for xml in [fixture::XML.replace("base_Class=\"A\"","base_Class=\"missing\""),fixture::XML.replace("<sysml:Block xmi:id=\"blockA\" base_Class=\"A\"/>","<sysml:Block xmi:id=\"blockA\" base_Class=\"A\"/><sysml:Block xmi:id=\"blockB\" base_Class=\"A\"/>")]{refused(&xml,"BLOCK_DECLARATION_UNSUPPORTED");}refused(&fixture::XML.replace("base_Class=\"A\"","base_Class=\"A\" isEncapsulated=\"true\""),"UNSUPPORTED_ATTRIBUTE");refused(&fixture::XML.replace("http://www.omg.org/spec/SysML/20181001/SysML","urn:unsupported:SysML"),"UNSUPPORTED_ELEMENT");
}
#[test]
fn source_admission_source_primary_or_descriptor_change_refuses_apply_without_partial_target_raw_allocator_or_history_changes(){
 for descriptor in [false,true]{let(root,path,mut e,p)=prepared(fixture::XML);ready(&p);let reviewed=p.review(&p.summary().required_acknowledgements).unwrap();let changed=if descriptor{path}else{root.path().join("source.xmi")};let mut bytes=std::fs::read(&changed).unwrap();bytes.push(b' ');std::fs::write(&changed,bytes).unwrap();let before=e.snapshot.clone();assert!(reviewed.apply(&mut e).unwrap_err().contains("SOURCE_CHANGED"));assert_eq!(e.snapshot,before);assert!(!e.can_undo());assert!(p.summary().ready_for_review);}
}
#[test]
fn source_admission_native_raw_or_foreign_runtime_changes_stale_the_sealed_review_even_when_all_source_files_still_match(){
 for foreign in [false,true]{let(_r,_path,mut e,p)=prepared(fixture::XML);ready(&p);let reviewed=p.review(&p.summary().required_acknowledgements).unwrap();if foreign{e=Editor::new(e.snapshot,"foreign:runtime".into()).unwrap();}else{e.command(&e.freshness(),Command::EditText{key:RawKey{record:RecordId(1003),field:"criterion".into(),item:None},text:"9e-".into()}).unwrap();}assert!(!p.target_current(&e));let before=e.snapshot.clone();assert!(reviewed.apply(&mut e).is_err());assert_eq!(e.snapshot,before);}
}
#[test]
fn source_admission_member_primary_companion_archive_and_native_migration_refusals_remain_explicit(){
 let(root,path,set)=source(fixture::XML);let e=editor();assert!(!a::prepare(set.clone(),"missing",&e,RecordId(1)).summary().ready_for_review);assert!(!a::prepare(set.clone(),&"x".repeat(1024),&e,RecordId(1)).summary().ready_for_review);let mut m:sets::Manifest=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();m.members[0].companions.push(sets::CompanionSpec{role:sets::CompanionRole::Opaque,file:sets::FileSpec{path:"missing.txt".into(),sha256:"0".repeat(64)}});std::fs::write(&path,serde_json::to_vec(&m).unwrap()).unwrap();assert!(!a::prepare(Arc::new(ProjectSet::open(&path).unwrap()),"supplier",&e,RecordId(1)).summary().ready_for_review);m.members[0].companions.clear();m.members[0].primary.sha256="0".repeat(64);std::fs::write(&path,serde_json::to_vec(&m).unwrap()).unwrap();assert!(!a::prepare(Arc::new(ProjectSet::open(&path).unwrap()),"supplier",&e,RecordId(1)).summary().ready_for_review);
 {
 use std::io::{Cursor,Write};use sha2::{Digest,Sha256};let area=tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();let mut zip=zip::ZipWriter::new(Cursor::new(Vec::new()));zip.start_file("com.nomagic.magicdraw.uml_model.model",zip::write::FileOptions::default()).unwrap();zip.write_all(fixture::XML.as_bytes()).unwrap();let bytes=zip.finish().unwrap().into_inner();let primary=area.path().join("source.mdzip");std::fs::write(&primary,&bytes).unwrap();let manifest=sets::Manifest{profile:sets::PROFILE.into(),schema_version:1,members:vec![sets::MemberSpec{key:"archive".into(),format:sets::Format::SemanticV2,primary:sets::FileSpec{path:"source.mdzip".into(),sha256:format!("{:x}",Sha256::digest(&bytes))},companions:vec![]}]};let path=area.path().join("archive.mbse-projects.json");std::fs::write(&path,serde_json::to_vec(&manifest).unwrap()).unwrap();let p=a::prepare(Arc::new(ProjectSet::open(&path).unwrap()),"archive",&e,RecordId(1));assert!(!p.summary().ready_for_review);assert!(p.summary().notices.iter().any(|n|n.code=="BOUNDED_COMPLETE_XML_INSPECTION_REQUIRED"));
 }
 let legacy=Editor::new(Snapshot::new(DocumentId("legacy:source-target".into())),"legacy:runtime".into()).unwrap();let p=a::prepare(set,"supplier",&legacy,RecordId(1));assert!(!p.summary().ready_for_review);assert!(p.summary().notices.iter().any(|n|n.code=="NATIVE_PAYLOAD_REFUSED"));assert!(root.path().exists());
}
#[test]
fn source_admission_xml_byte_element_depth_notice_and_processing_instruction_limits_refuse_without_truncating_success(){
 let huge=fixture::XML.replacen("<!-- Synthetic",&format!("<!--{}Synthetic"," ".repeat(a::MAX_XML_BYTES)),1);let(_r,_p,_e,p)=prepared(&huge);assert!(!p.summary().ready_for_review);assert!(p.summary().notices.iter().any(|n|n.code=="BOUNDED_COMPLETE_XML_INSPECTION_REQUIRED"));let many=fixture::XML.replacen("</uml:Model>",&format!("{}</uml:Model>","<untypedUnknown/>".repeat(513)),1);refused(&many,"XML_ELEMENT_OR_DEPTH_LIMIT");let deep=fixture::XML.replacen("</uml:Model>",&format!("{}{} </uml:Model>","<nested>".repeat(17),"</nested>".repeat(17)),1);refused(&deep,"XML_ELEMENT_OR_DEPTH_LIMIT");let notices=fixture::XML.replacen("</uml:Model>",&format!("{}</uml:Model>","<untypedUnknown/>".repeat(150)),1);let(_r,_p,_e,p)=prepared(&notices);assert!(!p.summary().ready_for_review&&p.summary().omitted_notices>0);refused(&fixture::XML.replacen("<uml:Model","<?untrusted never-execute?><uml:Model",1),"PROCESSING_INSTRUCTION_UNSUPPORTED");
}
#[test]
fn source_admission_apply_receipt_distinct_member_fresh_ids_single_undo_and_selected_system_preserve_original_bytes_and_whole_prior_meaning(){
 let(root,path,set)=source(fixture::XML);let manifest=std::fs::read(&path).unwrap();let mut e=editor();let before=e.snapshot.clone();let p=a::prepare(set.clone(),"supplier",&e,RecordId(60082));ready(&p);let applied=p.review(&p.summary().required_acknowledgements).unwrap().apply(&mut e).unwrap();assert!(applied.original_file_check.unchanged&&applied.checked_unix_milliseconds>0);assert_eq!(applied.target_snapshot_sha256,e.snapshot.digest());assert_eq!(e.snapshot.records,before.records);assert_eq!(e.snapshot.drafts,before.drafts);assert_eq!(e.snapshot.creation_drafts,before.creation_drafts);assert_eq!(e.snapshot.raw_version,before.raw_version);assert_eq!(e.snapshot.model_revision,before.model_revision+1);assert_eq!(e.snapshot.next_id,before.next_id+4);let store=t::load(&e.snapshot).unwrap();assert!(store.definitions.values().filter(|d|d.id.0>=before.next_id).all(|d|d.system==RecordId(60082)));let p=a::prepare(set.clone(),"consumer",&e,RecordId(60082));ready(&p);assert!(p.summary().mappings.iter().filter(|m|m.source_kind=="UmlClass").all(|m|m.native.0>=before.next_id+4));p.review(&p.summary().required_acknowledgements).unwrap().apply(&mut e).unwrap();let high=e.snapshot.next_id;assert!(e.undo_model(&e.freshness()).unwrap());assert_eq!(t::load(&e.snapshot).unwrap(),store);assert_eq!(e.snapshot.next_id,high);assert!(e.undo_model(&e.freshness()).unwrap());assert_eq!(t::load(&e.snapshot).unwrap(),t::load(&before).unwrap());assert_eq!(e.snapshot.records,before.records);assert_eq!(e.snapshot.drafts,before.drafts);assert_eq!(std::fs::read(root.path().join("source.xmi")).unwrap(),fixture::XML.as_bytes());assert_eq!(std::fs::read(&path).unwrap(),manifest);assert!(set.recheck().unwrap().unchanged);
}
#[test]
fn source_admission_truncated_inspection_exposes_omitted_loss_counts_and_cannot_review_a_partial_candidate(){
 let id="x".repeat(60000);let targets=(0..40).map(|i|format!("missing{i}")).collect::<Vec<_>>().join(" ");let xml=fixture::XML.replacen("xmi:id=\"part\"",&format!("xmi:id=\"{id}\""),1).replacen("type=\"B\"",&format!("type=\"{targets}\""),1);let(_root,_path,e,p)=prepared(&xml);let s=p.summary();assert!(!s.ready_for_review&&!s.inspection_details_omitted_budget);assert!(s.inspection_omitted_losses>0);assert_eq!(s.inspection_loss_count,s.source_inspection_losses.len());assert!(s.source_inspection_losses.iter().any(|l|l.code=="LOSSES_TRUNCATED"));assert!(s.native_candidate.is_none()&&s.mappings.is_empty());assert!(s.notices.iter().any(|n|n.code=="BOUNDED_COMPLETE_XML_INSPECTION_REQUIRED"&&n.blocking));assert!(p.review(&BTreeSet::new()).is_err());assert_eq!(e.snapshot,editor().snapshot);
}
#[test]
fn source_admission_checked_ui_bridge_uses_existing_native_views_history_and_preserves_layout_and_original_target_data(){
 use mbse_model::native_views::{Views,NativeAction};let(_r,_path,mut e,p)=prepared(fixture::XML);ready(&p);let before=e.snapshot.clone();let mut views=Views::new(&e).unwrap();let layout=views.document().clone();let checked=p.review(&p.summary().required_acknowledgements).unwrap().check_source().unwrap();let result=checked.apply_with_views(&mut e,&mut views).unwrap();assert_eq!(result.target_snapshot_sha256,e.snapshot.digest());assert_eq!(e.snapshot.records,before.records);assert_eq!(e.snapshot.drafts,before.drafts);assert_eq!(views.document(),&layout);let stamp=views.stamp(&e);views.dispatch(&mut e,&stamp,NativeAction::ModelUndo).unwrap();assert_eq!(t::load(&e.snapshot).unwrap(),t::load(&before).unwrap());assert_eq!(views.document(),&layout);
}
#[test]
fn source_admission_expired_completed_check_cannot_publish_native_state_after_a_delayed_ui_frame(){
 let(_r,_path,mut e,p)=prepared(fixture::XML);let checked=p.review(&p.summary().required_acknowledgements).unwrap().check_source().unwrap();let before=e.snapshot.clone();std::thread::sleep(std::time::Duration::from_millis(5100));assert!(checked.apply(&mut e).unwrap_err().contains("CHECK_EXPIRED"));assert_eq!(e.snapshot,before);assert!(!e.can_undo());
}
