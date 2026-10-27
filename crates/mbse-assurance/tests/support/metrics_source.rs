//! Source construction only, shared by rendered tests and the native fixture preparer.
use mbse_assurance::{Board,RequirementId,TraceKind,TraceEndpoint,RawDraft,traces};
use mbse_model::stage1::*;
use serde_json::{Value,json};
use std::collections::BTreeMap;
pub const SECOND_SYSTEM:u64=60082;
pub fn original()->Snapshot{let v:Value=serde_json::from_str(include_str!("../../../../docs/parity/p36b/verification/native-02/run/analysis-final-cold-copy.kernspan")).unwrap();serde_json::from_value(v["snapshot"].clone()).unwrap()}
pub fn model()->Snapshot{let mut s=original();assert_eq!(s.next_id,SECOND_SYSTEM);s.records.insert(RecordId(SECOND_SYSTEM),Record{id:RecordId(SECOND_SYSTEM),kind:RecordKind::System,name:"Independent metrics scope".into(),scope:RecordId(SECOND_SYSTEM),fields:BTreeMap::new()});s.next_id=SECOND_SYSTEM+1;s.model_revision+=1;s.validate().unwrap();s}
pub fn editor()->Editor{Editor::new(model(),"metrics-native-fixture:runtime".into()).unwrap()}
pub fn requirements(snapshot:&Snapshot)->Board{
 let mut b=Board::new();let a=b.add_identified_requirement("Supplier".into(),"METRIC-A".into(),None).unwrap();let child=b.add_identified_requirement("Selected child".into(),"METRIC-B".into(),Some(a)).unwrap();let c=b.add_identified_requirement("Other supplier".into(),"METRIC-C".into(),None).unwrap();assert_eq!((a,child,c),(RequirementId(1),RequirementId(2),RequirementId(3)));
 let case=b.add_test_case("Documentary case".into(),"Inspect retained source".into(),"Retained output".into()).unwrap();assert_eq!(case,4);
 for(kind,source,target)in[(TraceKind::Derive,TraceEndpoint::Requirement(child),a),(TraceKind::Refine,TraceEndpoint::Requirement(c),child),(TraceKind::Satisfy,traces::native_endpoint(snapshot,1).unwrap(),child),(TraceKind::Verify,TraceEndpoint::TestCase(case),child)]{b.add_trace(kind,source,target,"Display label only".into(),snapshot).unwrap();}
 let detached=b.add_trace(TraceKind::Satisfy,traces::native_endpoint(snapshot,SECOND_SYSTEM).unwrap(),c,"Detached documentary native source".into(),snapshot).unwrap();assert_eq!(detached,9);if let TraceEndpoint::Native(n)=&mut b.register.traces.get_mut(&detached).unwrap().source{n.document="different-metrics-source-document".into();}
 b.register.drafts.insert("2:statement".into(),RawDraft{text:String::new(),valid:false});b.register.unknown.insert("fixture.metrics.opaque/1".into(),json!({"raw":"9e","meaning":"Retained, not inventoried"}));b
}
