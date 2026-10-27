from pathlib import Path
import json,hashlib,gzip
r=Path(__file__).resolve().parents[4];d=r/'docs/parity/p39b';out=d/'review';rows=[];sha=lambda b:hashlib.sha256(b).hexdigest()
def archive(p,rel):
 raw=p.read_bytes();stored=gzip.compress(raw,mtime=0)if p.suffix=='.log'else raw;dest=out/rel
 if p.suffix=='.log':dest=Path(str(dest)+'.gz')
 dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(stored);rows.append(dict(original=str(p),path=str(dest.relative_to(out)),bytes=len(raw),sha256=sha(raw),stored_sha256=sha(stored)))
base=json.loads((d/'verification/compiler-01/RECEIPT.json').read_text());tests=[dict(slice=1,test=name)for cmd in base['commands']for name in cmd['passed_tests']];assert len(tests)==823
for i,jp in [(2,d/'attempts/02-review-output-hypothesis/JOB.json'),(3,d/'JOB.json')]:
 c=json.load(open(r/f'work/p39b-review-{i:02}.json'));assert c['job_sha256']==sha(jp.read_bytes())and not c['owned_jobs'];assert c['status']==('failed'if i==2 else'passed');archive(jp,Path(f'compiler-{i:02}/JOB.json'));archive(r/f'work/p39b-review-{i:02}.json',Path(f'compiler-{i:02}/RECEIPT.json'))
 for cmd in c['commands']:
  p=Path(cmd['log']);assert sha(p.read_bytes())==cmd['log_sha256'];archive(p,Path(f'compiler-{i:02}')/p.name);tests.extend(dict(slice=i,test=name)for name in cmd['passed_tests'])
 assert [len(cmd['passed_tests'])for cmd in c['commands']]==([0]if i==2 else[1,1,0])
 if i==3:assert sha(Path(c['preserved_binary']).read_bytes())==c['binary_sha256']
prior={v['test']for v in json.load(open(r/'docs/parity/p39a/verification/TEST_RESULTS.json'))['tests']};names={v['test']for v in tests};assert len(tests)==825 and len(names)==824 and len(names-prior)==15
(d/'verification/TEST_RESULTS.json').write_text(json.dumps(dict(compiler_slice_counts=[[823,0,0],[0],[1,1,0]],distinct_case_inventory=824,prior_case_names_reconciled=809,new_distinct_cases=sorted(names-prior),qualification='Original787-file0897f8a3 passes full823workspace/preparer/build. Isolatedreview test-only846ad3f6 new boundary hypothesis fails0passes: existing1MiBauxiliary loss budget refuses before adapter2MiBsummary could trigger. Final787-file5371749e adds only2documentary existing-projection omission-count fields, corrects new case and strengthens happy-path count assertions; new+affected cases pass1+1, desktopbuild passes.824distinct source-qualified names/825passing executions; prior823 not replayed, no final-source full-suite claim.',tests=tests),indent=2)+'\n');(out/'ARCHIVES.json').write_text(json.dumps(rows,indent=2)+'\n');print(dict(review_artifacts=len(rows),base_artifacts=5,distinct=824,passing_executions=825))
