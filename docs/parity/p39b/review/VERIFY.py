"""Read-only exact-source/evidence/preservation audit; no compiler or native replay."""
from pathlib import Path
import json,hashlib,gzip,subprocess,datetime
r=Path(__file__).resolve().parents[4];d=r/'docs/parity/p39b';out=d/'review';sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_text())
def git(*args,cwd=r):return subprocess.check_output(['git','-C',str(cwd),'-c','core.worktree='+str(cwd),*args],text=True).strip()
m=load(d/'SOURCE_MANIFEST.json');assert len(m['files'])==787
names=sorted(p for p in git('ls-files','-z').split('\0')if p and(p in('Cargo.toml','Cargo.lock')or p.startswith(('crates/','vendor/'))));assert names==[f['path']for f in m['files']]
for f in m['files']:b=(r/f['path']).read_bytes();assert len(b)==f['bytes']and sha(b)==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']
old=load(d/'attempts/01-full-regression/SOURCE_MANIFEST.json');assert [f['path']for f,o in zip(m['files'],old['files'])if f!=o]==['crates/mbse-exchange/src/source_admission.rs','crates/mbse-exchange/tests/source_admission.rs']
for root,expected in [(d/'verification',5),(out,8)]:
 a=load(root/'ARCHIVES.json');assert len(a)==expected
 for row in a:
  stored=(root/row['path']).read_bytes();assert sha(stored)==row['stored_sha256'];raw=gzip.decompress(stored)if row['path'].endswith('.gz')else stored;assert len(raw)==row['bytes']and sha(raw)==row['sha256']
tests=[]
for i,folder,manifest in [(1,d/'verification/compiler-01',d/'attempts/01-full-regression/SOURCE_MANIFEST.json'),(2,out/'compiler-02',d/'attempts/02-review-output-hypothesis/SOURCE_MANIFEST.json'),(3,out/'compiler-03',d/'SOURCE_MANIFEST.json')]:
 j=load(folder/'JOB.json');c=load(folder/'RECEIPT.json');assert c['job_sha256']==sha((folder/'JOB.json').read_bytes())and not c['owned_jobs'];assert c['status']==('failed'if i==2 else'passed');assert j['source_sha256']==load(manifest)['source_sha256']and j['source_manifest_sha256']==sha(manifest.read_bytes());assert [len(x['passed_tests'])for x in c['commands']]==[[823,0,0],[0],[1,1,0]][i-1]
 for cmd in c['commands']:
  raw=gzip.decompress((folder/(Path(cmd['log']).name+'.gz')).read_bytes());assert sha(raw)==cmd['log_sha256'];tests+=cmd['passed_tests']
 if i==3:assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
t=load(d/'verification/TEST_RESULTS.json');assert len(tests)==825 and len(set(tests))==824 and sorted(tests)==sorted(v['test']for v in t['tests']);prior={v['test']for v in load(r/'docs/parity/p39a/verification/TEST_RESULTS.json')['tests']};assert len(prior)==809 and prior<=set(tests)and sorted(set(tests)-prior)==t['new_distinct_cases']and len(t['new_distinct_cases'])==15
for row in load(d/'verification/PREPARED_SOURCE_HASHES.json')['files']:p=d/row['path'];assert p.stat().st_size==row['bytes']and sha(p.read_bytes())==row['sha256']
for p,h in load(d/'verification/FIXTURE_HASHES.json').items():relative=Path(p).relative_to(r.parent/'m7-p39b-source-adapter');assert sha((r/relative).read_bytes())==h
available=removed=0
for w in load(d/'PRESERVATION_BEFORE.json')['worktrees']:
 p=Path(w['path'])
 if w['state']=='user_removed_directory':assert not p.exists();assert git('rev-parse',w['branch'])==w['head'];removed+=1
 else:assert git('rev-parse','HEAD',cwd=p)==w['head']and git('status','--porcelain',cwd=p)==w['status'];available+=1
candidate=r.parent/'m7-p39b-source-adapter';assert git('rev-parse','HEAD',cwd=candidate)=='80a0bfdd5c6c21ab775108dd8b5e8b1c22ee35b0'and git('status','--porcelain',cwd=candidate)=='';available+=1;assert available==112 and removed==18
result=dict(checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),candidate='80a0bfdd5c6c21ab775108dd8b5e8b1c22ee35b0',base='12219917c22029d4f191767579e53f0a7b426ae6',same_agent=True,independent_review=False,source_files=787,source_sha256=m['source_sha256'],source_qualified_distinct_cases=824,new_distinct_cases=15,passing_executions=825,base_artifacts=5,review_artifacts=8,available_checkouts_unchanged=available,user_removed_registered_heads_retained=removed,findings_addressed=['Expose existing source projection omitted-loss and omitted-diagnostic counts; preserve earlier refusal behavior.','Correct retained failed boundary hypothesis: existing1MiBauxiliary loss budget triggers before new2MiBsummary budget. Focused corrected/new+affected checks and desktopbuild pass.'],blocking_findings_remaining=[],native_ui_runs=0,final_source_full_suite_claim=False)
(out/'REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
