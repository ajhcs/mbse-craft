"""Read-only PR source/evidence/preservation audit; no compiler/native execution."""
from pathlib import Path
import json,hashlib,gzip,subprocess,datetime,copy
r=Path(__file__).resolve().parents[4];d=r/'docs/parity/p38b';out=d/'review';sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_text())
def git(*args,cwd=r):return subprocess.check_output(['git','-C',str(cwd),'-c','core.worktree='+str(cwd),*args],text=True).strip()
m=load(d/'SOURCE_MANIFEST.json');assert len(m['files'])==781
names=sorted(p for p in git('ls-files','-z').split('\0')if p and(p in('Cargo.toml','Cargo.lock')or p.startswith(('crates/','vendor/'))));assert names==[f['path']for f in m['files']]
for f in m['files']:b=(r/f['path']).read_bytes();assert len(b)==f['bytes']and sha(b)==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']
for previous in ['01','02']:
 old=load(d/f'attempts/{previous}/SOURCE_MANIFEST.json');changes=[f['path']for f,o in zip(m['files'],old['files'])if f!=o];assert changes==['crates/mbse-desktop/src/project_set_ui_tests.rs']
archives=load(d/'verification/ARCHIVES.json');assert len(archives)==74
for a in archives:
 stored=(d/'verification'/a['path']).read_bytes();assert sha(stored)==a['stored_sha256'];raw=gzip.decompress(stored)if a['path'].endswith('.gz')else stored;assert len(raw)==a['bytes']and sha(raw)==a['sha256']
tests=[]
for i in range(1,4):
 folder=d/f'verification/compiler-{i:02}';j=load(folder/'JOB.json');c=load(folder/'RECEIPT.json');assert c['job_sha256']==sha((folder/'JOB.json').read_bytes())and c['owned_jobs']==[]
 manifest=d/(f'attempts/{i:02}/SOURCE_MANIFEST.json'if i<3 else'SOURCE_MANIFEST.json');assert j['source_sha256']==load(manifest)['source_sha256']and j['source_manifest_sha256']==sha(manifest.read_bytes());assert c['status']==('passed'if i==3 else'failed')
 for cmd in c['commands']:
  raw=gzip.decompress((folder/(Path(cmd['log']).name+'.gz')).read_bytes());assert sha(raw)==cmd['log_sha256'];tests+=cmd['passed_tests']
 assert [len(x['passed_tests'])for x in c['commands']]==[[0],[290],[509,0,0]][i-1]
 if i==3:assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256'];binary_sha=c['binary_sha256']
t=load(d/'verification/TEST_RESULTS.json');assert len(tests)==len(set(tests))==799 and sorted(tests)==sorted(v['test']for v in t['tests']);prior={v['test']for v in load(r/'docs/parity/p38a/verification/TEST_RESULTS.json')['tests']};assert len(prior)==793 and prior<=set(tests)and sorted(set(tests)-prior)==t['new_distinct_cases']and len(t['new_distinct_cases'])==6
j=load(d/'NATIVE_JOB_01.json');c=load(d/'verification/native-01/RECEIPT.json');assert c['job_sha256']==sha((d/'NATIVE_JOB_01.json').read_bytes())and c['status']=='actions_completed_review_pending'and not c.get('action_errors');assert len(c['runs'])==2 and j['source_sha256']==m['source_sha256']and j['binary_sha256']==binary_sha
for relative,expected in j['script_hashes'].items():assert sha((r/relative).read_bytes())==expected
for path,expected in j['inputs'].items():relative=Path(path).relative_to(Path(load(d/'JOB.json')['cwd']));assert sha((r/relative).read_bytes())==expected
for run in c['runs']:assert run['normal_app_exit']==run['driver_exit']==0 and run['owned_processes_absent']and not(Path(run['run'])/'Xauthority').exists()
n=d/'verification/native-01';ui={}
for p in n.rglob('*.ui.json'):
 number=int(p.name[:2]);assert number not in ui;ui[number]=load(p)
assert set(ui)==set(range(17));assert len({a['snapshot_hash']for a in ui.values()})==1
states={i:a['source_project_sets']for i,a in ui.items()};assert len(states[0]['summary']['members'])==4
left=states[1]['selected'];right=states[2]['selected'];supplier=states[3]['selected'];consumer=states[4]['selected'];assert left['member']=='left'and right['member']=='right'and left!=right and left['identity']==right['identity'];assert supplier['member']=='supplier'and consumer['member']=='consumer'and supplier!=consumer and supplier['identity']==consumer['identity'];assert states[5]['last_explicit_check']['Ok']['unchanged'];assert not states[7]['last_explicit_check']['Ok']['unchanged'];assert states[7]['summary']==states[8]['summary']==states[9]['summary']==states[0]['summary'];assert states[8]['error']and states[7]['selected']==states[8]['selected']==states[9]['selected'];assert states[11]['summary']['members'][0]['primary']['state']=='Changed'and states[11]['summary']['members'][0]['native_top_level_records']is None and states[11]['summary']['members'][1]['native_top_level_records']==21;assert states[12]['summary']is None and states[14]['summary']is None;assert states[15]['summary']['source_set_digest']==states[0]['summary']['source_set_digest']and states[15]['selected']==supplier
members=states[0]['summary']['members'];assert members[0]['companions'][0]['bound_declaration_checked'];assert members[0]['companions'][1]['file']['state']=='Missing'and members[0]['companions'][2]['file']['state']=='Changed';assert members[0]['native_top_level_records']==members[1]['native_top_level_records']==21
fixture=d/'fixtures/native-prepared-01';original=load(fixture/'left/native.kernspan')['snapshot'];right_source=load(fixture/'right/native.kernspan')['snapshot'];expected=copy.deepcopy(original);expected['records']['1']['name']='Supplier System with the same native document and local ID';assert right_source==expected;assert (original['model_revision'],original['raw_version'],original['next_id'])==(253,516,60083)
views=load(fixture/'left/native.mbse-views.json');maps=['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams','scenario_diagrams']
for native in [n/'run/project-set-target-checkpoint.kernspan',n/'cold-run/project-set-target-cold-copy.kernspan']:
 assert load(native)['snapshot']==original;v=load(native.with_suffix('.mbse-views.json'));assert all(v[k]==views[k]for k in maps)
for path in [p for p in fixture.rglob('*')if p.is_file()]:assert (n/'portable-set'/path.relative_to(fixture)).read_bytes()==path.read_bytes()
assert not(n/'run/blocked-save.kernspan').exists()
pres=load(d/'PRESERVATION_BEFORE.json')['worktrees'];available=removed=0
for w in pres:
 p=Path(w['path'])
 if w['state']=='user_removed_directory':assert not p.exists();assert git('rev-parse',w['branch'])==w['head'];removed+=1
 else:assert git('rev-parse','HEAD',cwd=p)==w['head'];assert git('status','--porcelain',cwd=p)==w['status'];available+=1
candidate=r.parent/'m7-p38b-native-project-sets';assert git('rev-parse','HEAD',cwd=candidate)=='3574d81762d85fc1911f739fedd882ecc3e2f99f'and git('status','--porcelain',cwd=candidate)=='';available+=1;assert available==108 and removed==18
result=dict(checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),candidate='3574d81762d85fc1911f739fedd882ecc3e2f99f',base='857f3da749718a7a01ff2e950184152d9b2cb0eb',same_agent=True,independent_review=False,source_files=781,source_sha256=m['source_sha256'],source_qualified_cases=799,new_cases=6,artifacts=74,native_runs=2,native_controller_errors=0,whole_snapshot_and_eight_maps_exact=True,portable_digest_and_qualified_identity_exact=True,available_checkouts_unchanged=available,user_removed_registered_heads_retained=removed,bounded_blocking_findings=[],new_source_changes=False,new_compiler_or_native_runs=False)
(out/'REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
