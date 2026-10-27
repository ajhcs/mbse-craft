from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p34c';sha=lambda b:hashlib.sha256(b).hexdigest();m=json.load(open(d/'SOURCE_MANIFEST.json'));assert len(m['files'])==758
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='9a1c96fefb6bb6890ac745da0042239ed3f4e2bafe888e4248773f04d3a465b9'
a=json.load(open(d/'verification/ARCHIVES.json'));assert len(a)==94
for row in a:
 p=d/'verification'/row['path'];data=p.read_bytes();assert sha(data)==row['stored_sha256'];raw=gzip.decompress(data)if p.suffix=='.gz'else data;assert len(raw)==row['bytes']and sha(raw)==row['sha256'];assert Path(row['original']).read_bytes()==raw
original=json.load(open(d/'fixtures/native.kernspan'))['snapshot'];saved=json.load(open(d/'verification/native-01/run/scenario-checkpoint.kernspan'))['snapshot'];cold=json.load(open(d/'verification/native-02/run/scenario-final-cold-copy.kernspan'))['snapshot'];assert saved==cold;assert {k:saved[k]for k in ['model_revision','raw_version','next_id']}==dict(model_revision=139,raw_version=328,next_id=14012)
assert {k:v for k,v in saved.items()if k not in ['extensions','model_revision','raw_version','next_id']}=={k:v for k,v in original.items()if k not in ['extensions','model_revision','raw_version','next_id']}
ns=saved['extensions']['namespaces'];oldns=original['extensions']['namespaces'];assert set(ns)==set(oldns)
for k,v in oldns.items():
 if k in ['mbse-craft.types/1','mbse-craft.type-drafts/1']:assert {n:x for n,x in v.items()if n not in ['scenarios','scenario_forms']}=={n:x for n,x in ns[k].items()if n not in ['scenarios','scenario_forms']}
 else:assert ns[k]==v
s=ns['mbse-craft.types/1']['scenarios']['14000'];assert len(s['lifelines'])==3 and len(s['messages'])==5 and len(s['alternatives'])==1 and len(s['alternatives']['14006']['operands'])==2
forms=ns['mbse-craft.type-drafts/1']['scenario_forms'];assert len(forms)==13
for key,field,value in [('lifeline:14000:14001','participant','1005'),('message:14000:14004','target','99999'),('message:14000:14005','reply_to','14005'),('operand:14000:14007','guard',''),('message:14000:new','order','1')]:assert forms[key][field]==value
assert s['lifelines']['14001']['participant']==1004 and s['messages']['14004']['target']==14002 and s['messages']['14005']['reply_to']==14004 and s['alternatives']['14006']['operands']['14007']['guard']=='temperature > 50 C'
v0=json.load(open(d/'fixtures/native.mbse-views.json'));v1=json.load(open(d/'verification/native-01/run/scenario-checkpoint.mbse-views.json'));v2=json.load(open(d/'verification/native-02/run/scenario-final-cold-copy.mbse-views.json'));maps=['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams','scenario_diagrams'];assert all(v1[k]==v2[k]for k in maps);assert all(all(v1[k][id]==value for id,value in v0.get(k,{}).items())for k in maps);assert v2['profile']=='mbse-craft.native-views/8';assert v2['scenario_diagrams']['50']['occurrences']['51']['geometry']['x']==25. and v2['scenario_diagrams']['54']['occurrences']['55']['geometry']['x']==125.;assert v2['views']['50']['camera']!=v2['views']['54']['camera']
for i in [1,2]:
 j=json.load(open(d/f'verification/native-{i:02}/JOB.json'));receipt=json.load(open(d/f'verification/native-{i:02}/RECEIPT.json'));assert receipt['job_sha256']==sha((d/f'verification/native-{i:02}/JOB.json').read_bytes());assert receipt['status']=='actions_completed_review_pending'and not receipt.get('action_errors');assert all(run['normal_app_exit']==run['driver_exit']==0 and run['owned_processes_absent']for run in receipt['runs']);compiler=json.load(open(d/f'verification/compiler-{i+2:02}/RECEIPT.json'));assert compiler['binary_sha256']==j['binary_sha256'];assert json.load(open(d/f'verification/compiler-{i+2:02}/JOB.json'))['source_sha256']==j['source_sha256']
pre=json.load(open(d/'review/PRESERVATION_BEFORE.json'));pres=[]
for w in pre['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==110
result=dict(result='pass_bounded_same_agent_isolated_review',candidate_commit='d428bb8bf03d33d08ab44245dc422d502c27c627',base_commit='2feb199e5d9b033b8fa9692f97f412a97e6835fe',source_sha256=m['source_sha256'],source_files=758,artifacts_verified=94,whole_saved_snapshot_and_eight_maps_exact=True,available_prior_checkouts_unchanged=92,user_removed_directories_heads_retained=18,independent_review=False,new_compiler_or_native_runs=0,production_source_changes=0,limits='Native removal/inverse observed active sequence view54; all-map persistence/inverse core coverage in P34b. Missing/incompatible participant rendering covered by core/egui fixture; native refuses wrong binding while preserving accepted data. No broader native cross-platform/scale/engineering/full-parity acceptance. All56specs/integratedexits remain open.',checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(d/'review/REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');(d/'review/PRESERVATION_AFTER.json').write_text(json.dumps(dict(worktrees=pres),indent=2)+'\n');print(result)
