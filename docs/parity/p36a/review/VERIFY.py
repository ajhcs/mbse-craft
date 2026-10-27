from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p36a';sha=lambda b:hashlib.sha256(b).hexdigest();load=lambda p:json.loads(Path(p).read_text());m=load(d/'SOURCE_MANIFEST.json')
assert len(m['files'])==766
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='b53a835007f34bf2945881bc79f729b199341a50e96ac4a4ea26734ac457d117'
rows=load(d/'verification/ARCHIVES.json');assert len(rows)==11
for row in rows:
 p=d/'verification'/row['path'];stored=p.read_bytes();assert sha(stored)==row['stored_sha256'];raw=gzip.decompress(stored)if p.suffix=='.gz'else stored;assert len(raw)==row['bytes']and sha(raw)==row['sha256'];assert Path(row['original']).read_bytes()==raw
for i in [1,2,3]:
 folder=d/f'verification/compiler-{i:02}';j=load(folder/'JOB.json');c=load(folder/'RECEIPT.json');assert c['job_sha256']==sha((folder/'JOB.json').read_bytes());assert c['status']==('passed'if i==3 else'failed')
 if i==3:assert j['source_sha256']==m['source_sha256'];assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
old=load(d/'attempts/02/ORIGINAL_ORACLE.json');oracle=load(d/'fixtures/quantities-and-thresholds.json');normalized=json.loads(json.dumps(oracle))
for row in normalized['cases']:
 for k in ['expected_failure','expected_assessment_failure']:
  if row.get(k)=='UnitMismatch':row[k]='QuantityKindMismatch'
assert old==normalized
prepared=d/'fixtures/native-prepared-01';expected=load(prepared/'EXPECTED.json');assert len(expected['cases'])==16
assert [row['expected']for row in expected['cases'][:14]]==oracle['cases']
before=load(r/'docs/parity/p35b/verification/native-02/run/parametric-final-cold-copy.kernspan')['snapshot'];after=load(prepared/'native.kernspan')['snapshot']
extra={str(row['calculation'])for row in expected['cases']if row['calculation']!=1003};assert len(extra)==15
assert {k:v for k,v in after['records'].items()if k not in extra}==before['records']
assert all(after[k]==before[k]for k in ['document_id','drafts','creation_drafts'])
ns=after['extensions']['namespaces'];oldns=before['extensions']['namespaces'];assert ns.keys()==oldns.keys()
extra_specs={str(row['specification'])for row in expected['cases']}
for name,v in oldns.items():
 if name=='mbse-craft.types/1':
  stripped=json.loads(json.dumps(ns[name]));stripped['parametrics']={k:v for k,v in stripped['parametrics'].items()if k not in extra_specs};assert stripped==v
 elif name=='mbse-craft.type-drafts/1':
  assert {k:v for k,v in ns[name].items()if k!='parametric_forms'}=={k:v for k,v in v.items()if k!='parametric_forms'}
  assert all(ns[name]['parametric_forms'][k]==value for k,value in v['parametric_forms'].items())
 else:assert ns[name]==v
v0=load(r/'docs/parity/p35b/verification/native-02/run/parametric-final-cold-copy.mbse-views.json');v1=load(prepared/'native.mbse-views.json')
for k in ['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams','scenario_diagrams']:assert v0[k]==v1[k]
assert {k:after[k]for k in ['model_revision','raw_version','next_id']}==dict(model_revision=249,raw_version=506,next_id=60079)
tests=load(d/'verification/TEST_RESULTS.json');assert len({t['test']for t in tests['tests']})==755 and len(tests['new_distinct_cases'])==24
pres=[]
for w in load(d/'review/PRESERVATION_BEFORE.json')['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==116
result=dict(result='pass_bounded_same_agent_isolated_review',candidate_commit='b357c3fd9bf9923a31ddda4e47d919e29c9b8c53',base_commit='04acd00eb1ecb24e602a08550c3f35a416aa9f97',source_sha256=m['source_sha256'],source_files=766,artifacts_verified=11,original_native_raw_opaque_and_eight_maps_exact=True,prepared_source_contexts=16,available_prior_checkouts_unchanged=98,user_removed_directories_heads_retained=18,independent_review=False,new_compiler_or_native_runs=0,production_source_changes=0,limits='Core analysis only; source-only native preparation is not rendered acceptance. Existing native engine/criterion authority retained. All56broadspecs/integratedexits remainopen; no independentengineering/fullparity/platform/scale/production acceptance.',checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(d/'review/REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');(d/'review/PRESERVATION_AFTER.json').write_text(json.dumps(dict(worktrees=pres),indent=2)+'\n');print(result)
