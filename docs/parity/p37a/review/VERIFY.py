from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p37a';review=d/'review';sha=lambda b:hashlib.sha256(b).hexdigest();load=lambda p:json.loads(Path(p).read_text());m=load(review/'SOURCE_MANIFEST.json');assert len(m['files'])==770
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='5121510e713d739b9ef6bda9d62b41933ae2145d40c50a3b77096746a8059d72'
base=load(review/'BASE_SOURCE_MANIFEST.json');old={f['path']:f['sha256']for f in base['files']};assert [f['path']for f in m['files']if f['sha256']!=old[f['path']]]==['crates/mbse-assurance/src/metrics.rs','crates/mbse-assurance/tests/metrics.rs']
verified=0
for root,index in [(d/'verification',d/'verification/ARCHIVES.json'),(review,review/'ARCHIVES.json')]:
 for row in load(index):
  p=root/row['path'];stored=p.read_bytes();assert sha(stored)==row['stored_sha256'];raw=gzip.decompress(stored)if p.suffix=='.gz'else stored;assert len(raw)==row['bytes']and sha(raw)==row['sha256'];assert Path(row['original']).read_bytes()==raw;verified+=1
assert verified==16
c=load(review/'verification/RECEIPT.json');j=load(review/'verification/JOB.json');assert c['status']=='passed'and[len(x['passed_tests'])for x in c['commands']]==[1,1,0];assert c['job_sha256']==sha((review/'verification/JOB.json').read_bytes());assert j['source_sha256']==m['source_sha256'];assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
tests=load(review/'TEST_RESULTS.json');assert len({t['test']for t in tests['tests']})==771
for path,expected in j['input_hashes'].items():assert sha(Path(path).read_bytes())==expected
pres=[]
for w in load(review/'PRESERVATION_BEFORE.json')['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==120
result=dict(result='pass_bounded_same_agent_isolated_review_after_parent_boundary_fix',candidate_commit='329c599fde948aa4dec7e4088b440b2c100bc1e0',base_commit='ab8de3248367ad0901025f94a399cae058299325',source_sha256=m['source_sha256'],source_files=770,base_and_review_artifacts_verified=16,distinct_qualified_cases=771,new_review_cases=1,review_affected_cases=1,native_rendered_runs=0,available_prior_checkouts_unchanged=102,user_removed_directories_heads_retained=18,independent_review=False,production_files_changed=['crates/mbse-assurance/src/metrics.rs'],limits='Finalsource parent XORboundary fix passesnew+affectedsubsetcase/build; originalfullnativecount/cold/export andlegacy checksremainprior-source-qualified. No finalfullsuite/nativeUI/independentengineering/fullparity/platform/scale/productionacceptance. P37b actualnative workflowrequired; all56broadspecs/integratedexitsremainopen.',checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(review/'REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');(review/'PRESERVATION_AFTER.json').write_text(json.dumps(dict(worktrees=pres),indent=2)+'\n');print(result)
