use super::*;
use mbse_model::types::parametrics::analysis::Assessment;
fn expectations()->serde_json::Value {serde_json::from_str(include_str!("../../../docs/parity/p36a/fixtures/native-prepared-01/EXPECTED.json")).unwrap()}
fn analysis_app()->(tempfile::TempDir,Workspace,egui::Context){
    let (area,mut a,ctx)=app();
    let value:serde_json::Value=serde_json::from_str(include_str!("../../../docs/parity/p36a/fixtures/native-prepared-01/native.kernspan")).unwrap();
    a.session.editor=mbse_model::stage1::Editor::new(serde_json::from_value(value["snapshot"].clone()).unwrap(),a.session.editor.freshness().instance_id).unwrap();
    a.reset_type_context();a.pm_open();select_case(&mut a,"length_conversion");(area,a,ctx)
}
fn select_case(a:&mut Workspace,name:&str){
    let expected=expectations();let id=Id(expected["cases"].as_array().unwrap().iter().find(|r|r["name"]==name).unwrap()["specification"].as_u64().unwrap());
    let store=ty::load(&a.session.editor.snapshot).unwrap();let f=pm::accepted_form(&store.parametrics[&id],id).unwrap();a.pm_pick(f,None).unwrap();
}
fn controls(a:&mut Workspace,ctx:&egui::Context){settle(a,ctx);if !a.audit.contains_key("parametric:analysis:Prepare"){click(a,ctx,"parametric:Analysis controls");}}
fn run_selected(a:&mut Workspace,ctx:&egui::Context){controls(a,ctx);click(a,ctx,"parametric:analysis:Prepare");assert!(a.parametrics.analysis.program.is_some(),"{}",a.parametrics.analysis.note);click(a,ctx,"parametric:analysis:Run");assert!(a.parametrics.analysis.report.is_some());}
#[test]
fn rendered_parametric_analysis_all_independent_quantities_thresholds_and_explicit_failures_preserve_source_and_layout(){
    let(_area,mut a,ctx)=analysis_app();let source=a.session.editor.snapshot.clone();let layout=a.views.views.document().encoded().unwrap();
    for row in expectations()["cases"].as_array().unwrap(){
        select_case(&mut a,row["name"].as_str().unwrap());run_selected(&mut a,&ctx);
        let report=a.parametrics.analysis.report.as_ref().unwrap();let expected=&row["expected"];
        assert_eq!(report.specification.0,row["specification"].as_u64().unwrap());assert!(report.is_current(&a.session.editor)&&report.parametric_execution_performed&&!report.verification_written);
        if let Some(code)=expected.get("expected_failure"){assert_eq!(format!("{:?}",report.outcome.diagnostic().unwrap().code),code.as_str().unwrap());assert!(matches!(report.assessment,Assessment::Unavailable(_)));}
        else{let actual=report.outcome.value().unwrap().value;let value=expected["expected_value"].as_f64().unwrap();assert!((actual-value).abs()<=1e-12+value.abs()*1e-12);
            if let Some(value)=expected.get("expected_assessment"){assert_eq!(matches!(report.assessment,Assessment::Met),value.as_bool().unwrap());assert!(!matches!(report.assessment,Assessment::Unavailable(_)));}
            else if let Some(code)=expected.get("expected_assessment_failure"){assert!(matches!(&report.assessment,Assessment::Unavailable(d)if format!("{:?}",d.code)==code.as_str().unwrap()));}
            else{assert!(matches!(report.assessment,Assessment::Unavailable(_)));}
        }
        assert_eq!(a.session.editor.snapshot,source);assert_eq!(a.views.views.document().encoded().unwrap(),layout);
    }
}
#[test]
fn rendered_parametric_analysis_stale_same_counter_source_and_changed_scope_cannot_run_or_relabel_prior_result(){
    let(_area,mut a,ctx)=analysis_app();run_selected(&mut a,&ctx);let saved=serde_json::to_value(&a.parametrics.analysis.report).unwrap();
    select_case(&mut a,"speed");let stamp=a.views.views.stamp(&a.session.editor);assert!(a.pm_analysis_action(&stamp,AnalysisAction::Run).unwrap_err().contains("differs"));
    assert_eq!(a.pm_analysis_audit()["selected_scope"],false);assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),saved);
    select_case(&mut a,"length_conversion");a.session.editor.snapshot.records.get_mut(&mbse_model::stage1::RecordId(1003)).unwrap().name="Same counters changed source".into();settle(&mut a,&ctx);
    assert_eq!(a.pm_analysis_audit()["current"],false);let stamp=a.views.views.stamp(&a.session.editor);assert!(a.pm_analysis_action(&stamp,AnalysisAction::Run).unwrap_err().contains("STALE"));
    assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),saved);
    click(&mut a,&ctx,"parametric:analysis:Run");assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),saved);
}
#[test]
fn rendered_parametric_analysis_pending_selected_proposal_refuses_prepare_and_retains_previous_result(){
    let(_area,mut a,ctx)=analysis_app();run_selected(&mut a,&ctx);let old=serde_json::to_value(&a.parametrics.analysis.report).unwrap();
    fill(&mut a,&ctx,"calculation","1001");let source=a.session.editor.snapshot.clone();click(&mut a,&ctx,"parametric:analysis:Prepare");
    assert!(a.parametrics.analysis.program.is_none());assert!(a.parametrics.analysis.note.contains("PROPOSAL_PENDING"));assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),old);assert_eq!(a.session.editor.snapshot,source);
    let stamp=a.views.views.stamp(&a.session.editor);assert!(a.pm_analysis_action(&stamp,AnalysisAction::Run).unwrap_err().contains("Prepare"));
}
#[test]
fn rendered_parametric_analysis_foreign_owner_and_unadmitted_text_disable_preparation_without_losing_report(){
    let(_area,mut a,ctx)=analysis_app();run_selected(&mut a,&ctx);let old=serde_json::to_value(&a.parametrics.analysis.report).unwrap();let source=a.session.editor.snapshot.clone();
    a.parametrics.owner.as_mut().unwrap().instance="foreign".into();let stamp=a.views.views.stamp(&a.session.editor);assert!(a.pm_analysis_action(&stamp,AnalysisAction::Prepare).is_err());
    settle(&mut a,&ctx);click(&mut a,&ctx,"parametric:analysis:Prepare");assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),old);
    a.parametrics.owner=Some(a.pm_owner());a.parametrics.pending=Some(Pending{owner:a.pm_owner(),form:a.parametrics.current.clone().unwrap()});let stamp=a.views.views.stamp(&a.session.editor);assert!(a.pm_analysis_action(&stamp,AnalysisAction::Prepare).is_err());assert_eq!(a.session.editor.snapshot,source);assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),old);
}
#[test]
fn rendered_parametric_analysis_copy_emits_exact_historical_report_and_clear_keeps_native_and_views(){
    let(_area,mut a,ctx)=analysis_app();run_selected(&mut a,&ctx);let report=serde_json::to_value(a.parametrics.analysis.report.as_ref().unwrap()).unwrap();let source=a.session.editor.snapshot.clone();let layout=a.views.views.document().encoded().unwrap();
    let r=rect(&a,"parametric:analysis:Copy");let pos=egui::pos2((r[0]+r[2])/2.,(r[1]+r[3])/2.);let mut copies=Vec::new();
    for pressed in[true,false]{let out=ctx.run_ui(egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(2200.,1600.))),focused:true,events:vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}],..Default::default()},|ui|{a.canvas_body(ui);a.pm_dialog(&ctx);});for c in &out.platform_output.commands{if let egui::OutputCommand::CopyText(text)=c{copies.push(text.clone());}}out.drop_without_applying_deltas();}
    assert_eq!(serde_json::from_str::<serde_json::Value>(copies.last().expect("actual copy output")).unwrap(),report);click(&mut a,&ctx,"parametric:analysis:Clear");assert!(a.parametrics.analysis.program.is_none()&&a.parametrics.analysis.report.is_none());assert_eq!(a.session.editor.snapshot,source);assert_eq!(a.views.views.document().encoded().unwrap(),layout);
}
#[test]
fn rendered_parametric_analysis_close_reopen_retains_session_report_but_source_load_reset_drops_authority(){
    let(_area,mut a,ctx)=analysis_app();run_selected(&mut a,&ctx);let source=a.session.editor.snapshot.clone();let report=serde_json::to_value(&a.parametrics.analysis.report).unwrap();
    click(&mut a,&ctx,"parametric:Close");assert!(!a.parametrics.open);a.pm_open();assert_eq!(serde_json::to_value(&a.parametrics.analysis.report).unwrap(),report);
    let stamp=a.views.views.stamp(&a.session.editor);let old_program=a.parametrics.analysis.program.clone().unwrap();a.session.editor=mbse_model::stage1::Editor::new(source.clone(),"cold-other-runtime".into()).unwrap();assert!(a.pm_analysis_action(&stamp,AnalysisAction::Run).is_err());assert!(!old_program.is_current(&a.session.editor));
    a.reset_type_context();assert!(a.parametrics.analysis.program.is_none()&&a.parametrics.analysis.report.is_none());assert_eq!(a.session.editor.snapshot,source);
}
