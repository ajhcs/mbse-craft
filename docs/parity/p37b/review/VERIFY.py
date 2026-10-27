from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p37b';sha=lambda b:hashlib.sha256(b).hexdigest();load=lambda p:json.loads(Path(p).read_text());m=load(d/'SOURCE_MANIFEST.json');assert len(m['files'])==774
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='b5279680ed752890383471c19a107c080dd28bea080b149edd12916802f8d7b8'
old=load(d/'attempts/01-full-regression-and-native/SOURCE_MANIFEST.json');oldrows={f['path']:f['sha256']for f in old['files']};assert [f['path']for f in m['files']if f['sha256']!=oldrows[f['path']]]==['crates/mbse-desktop/src/assurance_ui/metrics_ui.rs']
rows=load(d/'verification/ARCHIVES.json');assert len(rows)==83
for row in rows:
 p=d/'verification'/row['path'];stored=p.read_bytes();assert sha(stored)==row['stored_sha256'];raw=gzip.decompress(stored)if p.suffix=='.gz'else stored;assert len(raw)==row['bytes']and sha(raw)==row['sha256'];assert Path(row['original']).read_bytes()==raw
compilers=[load(d/f'verification/compiler-{i:02}/RECEIPT.json')for i in [1,2]];assert [[len(c['passed_tests'])for c in r['commands']]for r in compilers]==[[779,0,0],[0]]
for i,c in enumerate(compilers,1):
 j=load(d/f'verification/compiler-{i:02}/JOB.json');assert c['status']=='passed';assert c['job_sha256']==sha((d/f'verification/compiler-{i:02}/JOB.json').read_bytes());assert j['source_sha256']==(old['source_sha256']if i==1 else m['source_sha256']);assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
for i in [1,2]:
 folder=d/f'verification/native-{i:02}';j=load(folder/'JOB.json');c=load(folder/'RECEIPT.json');assert c['status']=='actions_completed_review_pending'and not c.get('action_errors');assert c['job_sha256']==sha((folder/'JOB.json').read_bytes());assert len(c['runs'])==(1 if i==1 else 2);assert j['source_sha256']==(old['source_sha256']if i==1 else m['source_sha256']);assert j['binary_sha256']==compilers[i-1]['binary_sha256']
 for run in c['runs']:assert run['normal_app_exit']==run['driver_exit']==0 and run['owned_processes_absent'];assert not(Path(run['run'])/'Xauthority').exists()
 for path,expected in j['inputs'].items():assert sha(Path(path).read_bytes())==expected
base=d/'fixtures/native-prepared-01';before=load(base/'native.kernspan')['snapshot'];saved=load(d/'verification/native-01/run/metrics-checkpoint.kernspan')['snapshot'];cold=load(d/'verification/native-02/run/metrics-final-cold-copy.kernspan')['snapshot'];assert before==saved==cold;assert {k:saved[k]for k in ['model_revision','raw_version','next_id']}==dict(model_revision=253,raw_version=516,next_id=60083)
register=load(base/'requirements.json')['register'];expected=json.loads(json.dumps(register));expected['drafts']['2:statement']['text']=' ';expected['revision']=10
assert load(d/'verification/native-01/run/metrics-register-checkpoint.json')['register']==load(d/'verification/native-02/run/metrics-final-cold-register.json')['register']==expected
views=[load(p)for p in [base/'native.mbse-views.json',d/'verification/native-01/run/metrics-checkpoint.mbse-views.json',d/'verification/native-02/run/metrics-final-cold-copy.mbse-views.json']]
for k in ['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams','scenario_diagrams']:assert views[0][k]==views[1][k]==views[2][k]
run=d/'verification/native-01/run';audits=[load(p)for p in sorted(run.glob('*.ui.json'))];assert len(audits)==16;assert len({a['snapshot_hash']for a in audits})==1
def metrics(index):return audits[index]['requirements_authoring']['metrics']
assert metrics(0)['report']['model']['top_level_records']==21 and metrics(0)['report']['traces']['traces']==5
assert metrics(1)['report']['model']['top_level_records']==1 and all(n==0 for n in metrics(1)['report']['model']['typed'].values())and metrics(1)['report']['traces']['traces']==5
assert metrics(2)['report']['model']['top_level_records']==20
assert tuple(metrics(3)['report']['traces'][k]for k in ['requirements','traces','active_traces','boundary_traces','parent_links_crossing_scope'])==(1,2,2,2,1)
assert tuple(metrics(4)['report']['traces'][k]for k in ['requirements','traces','boundary_traces','parent_links_crossing_scope'])==(1,0,1,1)
assert metrics(5)['report']['traces']['requirements']==0 and metrics(5)['report']['traces']['traces']==0
assert metrics(6)['report']['traces']is None and metrics(6)['report']['register_source']is None
assert all(not metrics(i)['current']for i in [7,8,9,11]);assert metrics(9)['ids']=='01'and metrics(9)['scope_error'];assert metrics(12)['report']['register_source']['sha256']!=metrics(10)['report']['register_source']['sha256'];assert metrics(13)['report']is None
cold_audits=[load(p)for p in sorted((d/'verification/native-02/run').glob('*.ui.json'))];assert len(cold_audits)==4 and len({a['snapshot_hash']for a in cold_audits})==1;assert cold_audits[0]['requirements_authoring']['metrics']['report']is None
assert all(not item['enabled']for item in cold_audits[0]['command_catalog']['items']if item['id']in ['edit.undo','edit.redo','view.layout_undo','view.layout_redo'])
final=cold_audits[1]['requirements_authoring']['metrics'];assert final['current']and final['report']['model']['top_level_records']==1 and final['report']['traces']['traces']==2
legacy=[load(p)for p in sorted((d/'verification/native-02/legacy-run').glob('*.ui.json'))];assert len(legacy)==3 and len({a['snapshot_hash']for a in legacy})==1;assert all(a['native_extensions']is None for a in legacy);assert legacy[0]['requirements_authoring']['metrics']['report']['model']['typed']is None and legacy[0]['requirements_authoring']['metrics']['report']['traces']['requirements']==0;assert legacy[1]['requirements_authoring']['metrics']['report']['traces']is None
assert load(d/'verification/native-02/legacy-run/legacy-metrics-native.kernspan')['snapshot'].get('extensions')is None
assert len({t['test']for t in load(d/'verification/TEST_RESULTS.json')['tests']})==779
pres=[]
for w in load(d/'review/PRESERVATION_BEFORE.json')['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==122
result=dict(result='pass_bounded_same_agent_isolated_review',candidate_commit='6ccd5f0682d6945b5abeb41102f7890d06e2b5cf',base_commit='af102fae44b1e4ed62ef22ea740ea1868a669ffa',source_sha256=m['source_sha256'],source_files=774,artifacts_verified=83,distinct_qualified_cases=779,new_gui_parser_behaviors=8,native_jobs=2,native_sessions=3,native_controller_errors=0,whole_native_register_eight_map_preservation_verified=True,only_register_intentional_raw_revision_change=True,cold_report_absent=True,legacy_no_migration=True,available_prior_checkouts_unchanged=104,user_removed_directories_heads_retained=18,independent_review=False,new_compiler_or_native_runs=0,production_source_changes=0,limits='Original26d full779/preparer/build/native01; finalB527 display-only strings/build/native02cold+legacy. No finalsourcefullsuite. Exactnativeclipboardpayloadnotread; actualGUIoutputtestcoversit. No evaluator/verificationexecutionbyquery; baselineStage1/evidenceassessmentseparate. All56broadspecs/integratedM06exit/fullparity/independentengineering/platform/scale/productionacceptanceopen.',checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(d/'review/REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');(d/'review/PRESERVATION_AFTER.json').write_text(json.dumps(dict(worktrees=pres),indent=2)+'\n');print(result)
