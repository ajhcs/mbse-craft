from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p36b';sha=lambda b:hashlib.sha256(b).hexdigest();load=lambda p:json.loads(Path(p).read_text());m=load(d/'SOURCE_MANIFEST.json')
assert len(m['files'])==768
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='02e376be985d4ab5234cefc39bf7aaac384cb2103352325e63bbd9740bcbf317'
rows=load(d/'verification/ARCHIVES.json');assert len(rows)==93
for row in rows:
 p=d/'verification'/row['path'];stored=p.read_bytes();assert sha(stored)==row['stored_sha256'];raw=gzip.decompress(stored)if p.suffix=='.gz'else stored;assert len(raw)==row['bytes']and sha(raw)==row['sha256'];assert Path(row['original']).read_bytes()==raw
compiler=load(d/'verification/compiler-02/RECEIPT.json');assert compiler['status']=='passed' and len(compiler['commands'][0]['passed_tests'])==761
assert sha(Path(compiler['preserved_binary']).read_bytes())==compiler['binary_sha256']
for i in [1,2]:
 folder=d/f'verification/native-{i:02}';j=load(folder/'JOB.json');c=load(folder/'RECEIPT.json');assert c['job_sha256']==sha((folder/'JOB.json').read_bytes());assert c['status']=='actions_completed_review_pending';assert len(c.get('action_errors',[]))==(3 if i==1 else 0)
 assert j['source_sha256']==m['source_sha256'] and j['binary_sha256']==compiler['binary_sha256']
 for run in c['runs']:assert run['normal_app_exit']==run['driver_exit']==0 and run['owned_processes_absent'];assert not(Path(run['run'])/'Xauthority').exists()
 for file,expected in j['inputs'].items():assert sha(Path(file).read_bytes())==expected
base=r/'docs/parity/p36a/fixtures/native-prepared-01';before=load(base/'native.kernspan')['snapshot'];saved=load(d/'verification/native-01/run/analysis-checkpoint.kernspan')['snapshot'];cold=load(d/'verification/native-02/run/analysis-final-cold-copy.kernspan')['snapshot'];assert saved==cold
assert {k:saved[k]for k in ['model_revision','raw_version','next_id']}==dict(model_revision=252,raw_version=516,next_id=60082)
assert all(before[k]==saved[k]for k in ['document_id','records','drafts','creation_drafts'])
oldns=before['extensions']['namespaces'];ns=saved['extensions']['namespaces'];assert oldns.keys()==ns.keys()
for key,value in oldns.items():
 if key=='mbse-craft.types/1':
  stripped=json.loads(json.dumps(ns[key]));owned=stripped['parametrics'].pop('60079');assert stripped==value
  assert owned['calculation']==60000 and owned['parameters']['60080']['name']=='d' and owned['bindings']['60081']['source']==dict(kind='NativeInput',calculation=60000,name='d')
 elif key=='mbse-craft.type-drafts/1':
  assert {k:v for k,v in ns[key].items()if k!='parametric_forms'}=={k:v for k,v in value.items()if k!='parametric_forms'}
  assert all(ns[key]['parametric_forms'][k]==v for k,v in value['parametric_forms'].items());assert len(ns[key]['parametric_forms'])==len(value['parametric_forms'])+3
  assert ns[key]['parametric_forms']['parametric:60079']['calculation']=='1001'
 else:assert ns[key]==value
views=[load(p)for p in [base/'native.mbse-views.json',d/'verification/native-01/run/analysis-checkpoint.mbse-views.json',d/'verification/native-02/run/analysis-final-cold-copy.mbse-views.json']]
for k in ['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams','scenario_diagrams']:assert views[0][k]==views[1][k]==views[2][k]
cases=load(base/'EXPECTED.json')['cases'];run=d/'verification/native-01/run';source_hashes=[]
for i,row in enumerate(cases):
 audit=load(run/('%02d-%s-native-quantity-assessment.ui.json'%(i+1,row['name'])));p=audit['parametrics']['analysis'];report=p['report'];source_hashes.append(audit['snapshot_hash']);assert report['specification']==row['specification']and p['current']and p['selected_scope']and not report['verification_written'];e=row['expected']
 if 'expected_failure'in e:assert report['outcome']['Failure']['code']==e['expected_failure']and 'Unavailable'in report['assessment']
 else:
  v=report['outcome']['Value'];want=e['expected_value'];assert abs(v['value']-want)<=1e-12+abs(want)*1e-12
  if 'output'in e:assert v['quantity']['unit']==e['output']
  if 'expected_assessment'in e:assert report['assessment']==('Met'if e['expected_assessment']else'NotMet')
  elif 'expected_assessment_failure'in e:assert report['assessment']['Unavailable']['code']==e['expected_assessment_failure']
  else:assert 'Unavailable'in report['assessment']
assert len(set(source_hashes))==1
cold_audits=[load(p)for p in sorted((d/'verification/native-02/run').glob('*.ui.json'))];assert len(cold_audits)==5;assert len({a['snapshot_hash']for a in cold_audits})==1
assert cold_audits[0]['parametrics']['analysis']['program']is None and cold_audits[0]['parametrics']['analysis']['report']is None
assert all(not item['enabled']for item in cold_audits[0]['command_catalog']['items']if item['id']in ['edit.undo','edit.redo','view.layout_undo','view.layout_redo'])
assert all(a['parametrics']['analysis']['program']is None and a['parametrics']['analysis']['report']is None for a in cold_audits)
assert 'PROPOSAL_PENDING'in cold_audits[1]['parametrics']['analysis']['note']
pres=[]
for w in load(d/'review/PRESERVATION_BEFORE.json')['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==118
result=dict(result='pass_bounded_same_agent_isolated_review',candidate_commit='853413f6c0f7ce462b79d2c20c144d27447caba3',base_commit='39659c97d02381211854042452a9e47b461e6a15',source_sha256=m['source_sha256'],source_files=768,artifacts_verified=93,workspace_cases=761,new_gui_behaviors=6,native_contexts=16,native_controller_errors=[3,0],whole_native_prior_meaning_and_eight_maps_preserved=True,cold_program_and_report_absent=True,available_prior_checkouts_unchanged=100,user_removed_directories_heads_retained=18,independent_review=False,new_compiler_or_native_runs=0,production_source_changes=0,limits='Three native01controller timingerrors retained; failed/unexecuted remainder passed with stablegeometry/explicitselection/frame settlement. NativeOSclipboardpayloadnotread; actualGUIoutputtestcoverspayload. No independentengineering/fullparity/platform/scale/production acceptance; all56broadspecs/integratedexitsremainopen.',checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(d/'review/REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');(d/'review/PRESERVATION_AFTER.json').write_text(json.dumps(dict(worktrees=pres),indent=2)+'\n');print(result)
