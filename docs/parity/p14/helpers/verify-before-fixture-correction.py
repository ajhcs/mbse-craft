#!/usr/bin/env python3
"""Frozen P14 CLI acceptance, no work on import; owned output on /workspace only."""
import argparse,datetime,fcntl,hashlib,importlib.util,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--job',type=Path,required=True);a=ap.parse_args();job=json.loads(a.job.read_text());exe=Path(job['binary']);assert digest(exe)==job['binary_sha256'];assert digest(Path(__file__))==job['verifier_sha256']
 for p,h in job['inputs'].items():assert digest(Path(p))==h,p
 output=Path(job['output']);assert output.is_relative_to('/workspace/build/builderbob/mbse-craft/milestone1');output.mkdir(exist_ok=False)
 lease=Path('/workspace/build/builderbob/mbse-craft/milestone1/target.lock').open('a');fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
 spec=importlib.util.spec_from_file_location('local_preflight',ROOT/'scripts/run-p02-local.py');check=importlib.util.module_from_spec(spec);spec.loader.exec_module(check)
 receipt=dict(schema='mbse-craft.p14-cli-acceptance/1',job_sha256=digest(a.job),status='active',started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),cases=[])
 def run(name,source,destination,success):
  before=digest(destination) if destination.exists() else None
  preflight=check.local_preflight(compiler_roots=job['compiler_roots']);argv=[str(exe),'--state-dir',str(output/'state'),'--migrate-extensions',str(source),'--destination',str(destination)];r=subprocess.run(argv,capture_output=True,check=False)
  (output/(name+'.stdout')).write_bytes(r.stdout);(output/(name+'.stderr.log')).write_bytes(r.stderr)
  row=dict(case=name,argv=argv,preflight=preflight,returncode=r.returncode,stdout_sha256=hashlib.sha256(r.stdout).hexdigest(),stderr_sha256=hashlib.sha256(r.stderr).hexdigest(),expected_success=success);receipt['cases'].append(row)
  assert (r.returncode==0)==success,(name,r.returncode,r.stderr.decode())
  if success:
   report=json.loads(r.stdout);value=json.loads(destination.read_text());old=json.loads(source.read_text());core=value['snapshot'].copy();extension=core.pop('extensions');assert core==old['snapshot'];assert value['profile']=='kernspan.stage1/2' and extension==dict(profile='mbse-craft.extensions/1',schema_version=1,required_namespaces=[],namespaces={});assert report['destination_sha256']==digest(destination) and not report['source_overwrite_allowed'];row.update(report=report,core_exact=True,profile_and_document_identity_preserved=True)
  elif before is not None:assert digest(destination)==before
  else:assert not destination.exists()
  return destination
 try:
  f=Path(job['fixture_directory']);migrated=run('01-current-with-opaque-drafts',f/'opaque-drafts.kernspan',output/'migrated.kernspan',True)
  run('02-legacy',f/'legacy.kernspan',output/'legacy-migrated.kernspan',True)
  run('03-future-schema',f/'future.kernspan',output/'future-refused.kernspan',False)
  run('04-required-future',f/'required-future.kernspan',output/'required-refused.kernspan',False)
  run('05-source-overwrite',f/'opaque-drafts.kernspan',f/'opaque-drafts.kernspan',False)
  run('06-occupied-target',f/'opaque-drafts.kernspan',migrated,False)
  run('07-already-versioned',migrated,output/'remigration-refused.kernspan',False)
  assert all(digest(Path(p))==h for p,h in job['inputs'].items());receipt.update(status='passed',original_inputs_unchanged=True,migrated_sha256=digest(migrated),migrated_snapshot_sha256=receipt['cases'][0]['report']['snapshot_sha256'])
 finally:
  receipt['ended_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();(output/'RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n');lease.close()
 print(json.dumps(dict(status=receipt['status'],cases=len(receipt['cases']),output=str(output))))
if __name__=='__main__':main()
