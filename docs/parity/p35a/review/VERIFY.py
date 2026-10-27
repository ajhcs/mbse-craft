from pathlib import Path
import json,gzip,hashlib,subprocess,datetime
r=Path.cwd();d=r/'docs/parity/p35a';review=d/'review';out=review/'verification';out.mkdir(exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest();m=json.load(open(review/'SOURCE_MANIFEST.json'));assert len(m['files'])==760
for f in m['files']:assert sha((r/f['path']).read_bytes())==f['sha256']
assert sha(json.dumps(m['files'],sort_keys=True,separators=(',',':')).encode())==m['source_sha256']=='0953707195245ec05f4c70d8c91d0daf4eb67d68baa717668e686b2ca2e0e032'
old={f['path']:f['sha256']for f in json.load(open(d/'SOURCE_MANIFEST.json'))['files']};assert [f['path']for f in m['files']if f['sha256']!=old[f['path']]]==['crates/mbse-model/tests/parametrics.rs']
original=json.load(open(d/'verification/ARCHIVES.json'));assert len(original)==10
for row in original:
 p=d/'verification'/row['path'];data=p.read_bytes();assert sha(data)==row['stored_sha256'];raw=gzip.decompress(data)if p.suffix=='.gz'else data;assert len(raw)==row['bytes']and sha(raw)==row['sha256']and Path(row['original']).read_bytes()==raw
j=json.load(open(review/'JOB.json'));c=json.load(open('work/p35a-review-01.json'));assert c['status']=='passed'and[len(cmd['passed_tests'])for cmd in c['commands']]==[1,0];assert c['job_sha256']==sha((review/'JOB.json').read_bytes());assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256'];rows=[]
def archive(p,rel):
 raw=p.read_bytes();stored=gzip.compress(raw,mtime=0)if p.suffix=='.log'else raw;dest=out/rel
 if p.suffix=='.log':dest=Path(str(dest)+'.gz')
 dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(stored);rows.append(dict(original=str(p),path=str(dest.relative_to(out)),bytes=len(raw),sha256=sha(raw),stored_sha256=sha(stored)))
archive(review/'JOB.json',Path('JOB.json'));archive(r/'work/p35a-review-01.json',Path('RECEIPT.json'))
for cmd in c['commands']:
 p=Path(cmd['log']);assert sha(p.read_bytes())==cmd['log_sha256'];archive(p,Path(p.name))
pres=[]
for w in json.load(open(review/'PRESERVATION_BEFORE.json'))['worktrees']:
 p=w['path']
 if w['state']=='user_removed_directory':assert not Path(p).exists();assert subprocess.check_output(['git','rev-parse',w['branch']],text=True).strip()==w['head'];pres.append(w|dict(committed_head_retained=True))
 else:assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'rev-parse','HEAD'],text=True).strip()==w['head'];assert subprocess.check_output(['git','-C',p,'-c','core.worktree='+p,'status','--porcelain'],text=True).strip()==w['status'];pres.append(w|dict(state='unchanged'))
assert len(pres)==112
for name,value in [('ARCHIVES.json',rows),('TEST_RESULTS.json',dict(compiler_slice_counts=[1,0],distinct_case_inventory=723,new_distinct_cases=[],tests=[dict(test=n,slice='review-01')for cmd in c['commands']for n in cmd['passed_tests']],qualification='One affected capacity fixture strengthens mixed511specs+1existing component at shared512; production unchanged; original723distinctcases retained. No summedfinalsourcefullsuite/nativeUI/evaluation claim.')),('PRESERVATION_AFTER.json',dict(worktrees=pres)),('REVIEW_AUDIT.json',dict(result='pass_bounded_same_agent_isolated_review',candidate_commit='50ab1aed4fc93b76d6891ad71622c8f6cecb2035',base_commit='e388d46d60d2c1fbf7f7cbca889d64b6804f91a7',source_files=760,source_sha256=m['source_sha256'],production_source_changes=0,review_fixture_changes=1,original_artifacts_recomputed=10,review_artifacts=len(rows),affected_cases=1,desktop_build_passed=True,distinct_case_inventory=723,native_parametric_runs=0,evaluation_performed=False,available_prior_checkouts_unchanged=94,user_removed_directories_heads_retained=18,independent_review=False,checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat()))]:
 (review/name).write_text(json.dumps(value,indent=2)+'\n')
print(dict(artifacts=len(rows),source=m['source_sha256'],affected=1,distinct=723))
