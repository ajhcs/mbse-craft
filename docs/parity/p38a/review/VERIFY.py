"""Read-only source/evidence/preservation audit; does not execute application tests."""
from pathlib import Path
import json,hashlib,gzip,subprocess,datetime
r=Path(__file__).resolve().parents[4];d=r/'docs/parity/p38a';out=d/'review';sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_text())
def git(*args,cwd=r):return subprocess.check_output(['git','-C',str(cwd),'-c','core.worktree='+str(cwd),*args],text=True).strip()
m=load(d/'SOURCE_MANIFEST.json');assert len(m['files'])==777
names=sorted(p for p in git('ls-files','-z').split('\0')if p and(p in('Cargo.toml','Cargo.lock')or p.startswith(('crates/','vendor/'))));assert names==[f['path']for f in m['files']]
for f in m['files']:b=(r/f['path']).read_bytes();assert len(b)==f['bytes']and sha(b)==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']
j=load(d/'JOB.json');assert j['source_sha256']==m['source_sha256']and j['source_manifest_sha256']==sha((d/'SOURCE_MANIFEST.json').read_bytes());assert sha((r/'Cargo.lock').read_bytes())==j['cargo_lock_sha256']
for path,expected in j['input_hashes'].items():
 original=Path(path);rel=original.relative_to(Path(j['cwd']));assert sha((r/rel).read_bytes())==expected
for a in load(d/'verification/ARCHIVES.json'):
 stored=(d/'verification'/a['path']).read_bytes();assert sha(stored)==a['stored_sha256'];raw=gzip.decompress(stored)if a['path'].endswith('.gz')else stored;assert len(raw)==a['bytes']and sha(raw)==a['sha256']
c=load(d/'verification/compiler-01/RECEIPT.json');assert c['status']=='passed'and c['owned_jobs']==[]and c['job_sha256']==sha((d/'JOB.json').read_bytes());assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
passed=[]
for command in c['commands']:
 assert command['exit_code']==0;log=d/'verification/compiler-01'/(Path(command['log']).name+'.gz');assert sha(gzip.decompress(log.read_bytes()))==command['log_sha256'];passed+=command['passed_tests']
t=load(d/'verification/TEST_RESULTS.json');assert len(passed)==len(set(passed))==793;assert sorted(passed)==sorted(v['test']for v in t['tests']);prior={v['test']for v in load(r/'docs/parity/p37b/verification/TEST_RESULTS.json')['tests']};assert len(prior)==779 and prior<=set(passed)and sorted(set(passed)-prior)==t['new_distinct_cases']and len(t['new_distinct_cases'])==14
pres=load(d/'PRESERVATION_BEFORE.json')['worktrees'];available=removed=0
for w in pres:
 p=Path(w['path'])
 if w['state']=='user_removed_directory':assert not p.exists();assert git('rev-parse',w['branch'])==w['head'];removed+=1
 else:assert git('rev-parse','HEAD',cwd=p)==w['head'];assert git('status','--porcelain',cwd=p)==w['status'];available+=1
candidate=r.parent/'m7-p38a-project-sets';assert git('rev-parse','HEAD',cwd=candidate)=='2458e7d7e499a9a22fc7aaab1b9fd04e13c926a1'and git('status','--porcelain',cwd=candidate)=='';available+=1
assert available==106 and removed==18;assert git('rev-parse','HEAD',cwd=Path('/workspace/repos/builderbob/mbse-craft'))=='53133f6ac228fac885767a5d9f1165a8db5d139b'
result=dict(checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),candidate='2458e7d7e499a9a22fc7aaab1b9fd04e13c926a1',base='a001ce274886a0b45535e7d6863aea76fa3e12af',same_agent=True,independent_review=False,source_files=777,source_sha256=m['source_sha256'],workspace_tests=793,new_cases=14,artifacts=4,available_checkouts_unchanged=available,user_removed_registered_heads_retained=removed,bounded_blocking_findings=[],new_source_changes=False,new_compiler_or_native_runs=False)
(out/'REVIEW_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
