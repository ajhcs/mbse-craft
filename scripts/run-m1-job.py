#!/usr/bin/env python3
"""Execute one frozen Milestone 1 job with bounded BuilderBob local preflights."""
import argparse
import datetime as dt
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def source_check(job):
    assert sha(Path(__file__)) == job['runner_sha256'], 'Runner changed'
    assert str(ROOT) == job['cwd'], 'Wrong isolated worktree'
    manifest_path = ROOT/job['source_manifest']
    assert sha(manifest_path) == job['source_manifest_sha256'], 'Manifest changed'
    manifest = json.loads(manifest_path.read_text())
    assert manifest['source_sha256'] == job['source_sha256']
    names = subprocess.check_output(['git','ls-files','-z'], cwd=ROOT, text=True).split('\0')
    names = sorted(p for p in names if p and (p in ('Cargo.toml','Cargo.lock') or p.startswith(('crates/','vendor/'))))
    assert names == [row['path'] for row in manifest['files']], 'Source file set changed'
    for row in manifest['files']:
        path = ROOT/row['path']
        assert path.stat().st_size == row['bytes'] and sha(path) == row['sha256'], row['path']
    assert sha(ROOT/'Cargo.lock') == job['cargo_lock_sha256'], 'Lock changed'
    assert job['env']['CARGO_BUILD_JOBS'] == job['env']['RUST_TEST_THREADS'] == '1'
    for path, expected in job.get('input_hashes', {}).items():
        assert sha(Path(path)) == expected, 'Job input changed: ' + path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--job', type=Path, required=True)
    parser.add_argument('--record', type=Path, required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    record_path = args.record.resolve()
    assert record_path.is_relative_to(ROOT) and not record_path.exists()
    record_path.parent.mkdir(parents=True,exist_ok=True)
    job = json.loads(args.job.read_text())
    receipt = dict(schema='mbse-craft.m1-local-job-receipt/1', job_sha256=sha(args.job),
                   started_utc=now(), status='preflight', commands=[], owned_jobs=[])
    def save():
        record_path.write_text(json.dumps(receipt,indent=2)+'\n')
    try:
        source_check(job)
        spec = importlib.util.spec_from_file_location('local_preflight', ROOT/'scripts/run-p02-local.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        assert sha(ROOT/'scripts/run-p02-local.py') == job['preflight_module_sha256']
        receipt['preflight'] = module.local_preflight(compiler_roots=job.get('compiler_roots'))
        if not args.execute:
            receipt['status'] = 'preflight_passed_no_execution'
            return 0
        assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True), 'Dirty candidate'
        output_root = Path('/workspace/build/builderbob/mbse-craft/milestone1')
        output_root.mkdir(parents=True,exist_ok=True)
        with (output_root/'target.lock').open('a') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            spent = output_root/('job-'+sha(args.job)+'.started')
            assert not spent.exists(), 'Job already spent; do not replay passes'
            output = output_root/(job['id']+'-'+dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ'))
            output.mkdir(exist_ok=False)
            receipt.update(output_directory=str(output),checkout_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip())
            (output/'JOB.json').write_bytes(args.job.read_bytes())
            env=os.environ.copy();env.update(job['env'])
            for key in ('TMPDIR','CARGO_TARGET_DIR'):
                path=Path(env[key]);assert path.is_relative_to(output_root);path.mkdir(parents=True,exist_ok=True)
            for index, command in enumerate(job['commands']):
                source_check(job)
                preflight=module.local_preflight(compiler_roots=job.get('compiler_roots'))
                log=output/f'{index:02}.log'
                entry=dict(index=index,argv=command,started_utc=now(),preflight=preflight,log=str(log))
                receipt['commands'].append(entry)
                if index==0:
                    with spent.open('x') as stream:stream.write(json.dumps(dict(receipt=str(record_path),output=str(output)))+'\n')
                with log.open('xb') as stream:
                    child=subprocess.Popen(command,cwd=ROOT,env=env,stdout=stream,stderr=subprocess.STDOUT)
                    entry.update(pid=child.pid,start_ticks=int(Path(f'/proc/{child.pid}/stat').read_text().rsplit(')',1)[1].split()[19]))
                    receipt.update(status='running',owned_jobs=[dict(pid=child.pid,start_ticks=entry['start_ticks'])]);save()
                    entry['exit_code']=child.wait()
                receipt['owned_jobs']=[]
                entry.update(ended_utc=now(),log_sha256=sha(log),passed_tests=re.findall(r'^test ([^\n]+) \.\.\. ok$',log.read_text(errors='replace'),re.M));save()
                if entry['exit_code']:
                    receipt['status']='failed'
                    return 1
            source_check(job)
            binary=Path(env['CARGO_TARGET_DIR'])/'debug/mbse-desktop';preserved=output/'mbse-desktop'
            with binary.open('rb') as src,preserved.open('xb') as dest:shutil.copyfileobj(src,dest)
            preserved.chmod(0o700)
            receipt.update(status='passed',binary_sha256=sha(preserved),binary_bytes=preserved.stat().st_size,preserved_binary=str(preserved))
            return 0
    except Exception as error:
        receipt.update(status='held_or_failed',reason=repr(error))
        return 2
    finally:
        receipt['ended_utc']=now();save()
        print(json.dumps(dict(status=receipt['status'],receipt=str(record_path),tests=[len(x.get('passed_tests',[]))for x in receipt['commands']],reason=receipt.get('reason'))))


if __name__=='__main__':
    sys.exit(main())
