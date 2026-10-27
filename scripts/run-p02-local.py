#!/usr/bin/env python3
"""Bounded BuilderBob P02 execution. No observer, daemon or historical grant.

Default: preflight only. --execute consumes the new local job once.
"""
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
JOB = ROOT / 'docs/parity/p02/admission/LOCAL_JOB.json'


def sha(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def need(value, reason):
    if not value:
        raise ValueError(reason)


def local_preflight(compiler_roots=None):
    workspace = json.loads(subprocess.check_output(['bob-admin', 'workspace-status'], text=True))
    need(workspace['outcome'] == 'ok', 'Workspace identity unavailable')
    need(workspace['device']['uuid'] == 'dcb87552-b22f-411c-bd86-4e2da80f1c6f', 'Wrong workspace')
    free = shutil.disk_usage(ROOT).free
    need(free >= 8 * 1024**3, 'Less than 8 GiB workspace reserve')
    processes, appservers, other_compilers = [], [], []
    memory = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    available_memory = int(memory['MemAvailable'].split()[0]) * 1024
    need(available_memory >= 4 * 1024**3, 'Less than 4 GiB available memory')
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        try:
            if path.stat().st_uid != os.getuid():
                continue
            comm = (path/'comm').read_text().strip()
            if comm in ('cargo', 'rustc', 'mbse-desktop'):
                cwd = os.readlink(path/'cwd')
                args = (path/'cmdline').read_bytes().decode(errors='replace').split('\0')
                destinations = [args[i+1] for i, arg in enumerate(args[:-1]) if arg in ('--out-dir', '--target-dir')]
                row = dict(pid=int(path.name), comm=comm, cwd=cwd, output_dirs=destinations)
                same_scope = compiler_roots is None or any(
                    Path(value).is_relative_to(Path(root)) for root in compiler_roots for value in [cwd, *destinations])
                (processes if comm == 'mbse-desktop' or same_scope else other_compilers).append(row)
            if comm != 'codex':
                continue
            args = (path/'cmdline').read_bytes().split(b'\0')
            if b'app-server' not in args:
                continue
            ticks = int((path/'stat').read_text().rsplit(')', 1)[1].split()[19])
            soft = int(re.search(r'^Max open files\s+(\d+)', (path/'limits').read_text(), re.M)[1])
            used = len(list((path/'fd').iterdir()))
            appservers.append(dict(pid=int(path.name), start_ticks=ticks,
                                   exe=os.readlink(path/'exe'), soft_nofile=soft, fd_count=used))
        except FileNotFoundError:
            continue
    need(not processes, 'Existing compiler/native process requires coordination: ' + json.dumps(processes))
    need(appservers, 'No visible app-server identity')
    for server in appservers:
        need(server['soft_nofile'] - server['fd_count'] >= 128 and
             server['fd_count'] < server['soft_nofile'] * .8, 'App-server descriptor reserve insufficient')
    return dict(schema='mbse-craft.builderbob-local-preflight/1', checked_utc=now(),
                uid=os.getuid(), boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                pid_namespace=os.readlink('/proc/self/ns/pid'), workspace=workspace,
                free_workspace_bytes=free, available_memory_bytes=available_memory,
                conflicting_processes=processes, other_project_compilers=other_compilers, appservers=appservers,
                observer_started=False, original_observer_admission_claimed=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--record', type=Path, required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    record_path = args.record.resolve()
    need(record_path.is_relative_to(ROOT) and not record_path.exists(), 'Use a fresh worktree receipt')
    record_path.parent.mkdir(parents=True, exist_ok=True)
    job = json.loads(JOB.read_text())
    receipt = dict(schema='mbse-craft.p02-local-execution/1', started_utc=now(),
                   job_sha256=sha(JOB), status='preflight', commands=[], owned_jobs=[])
    def save():
        record_path.write_text(json.dumps(receipt, indent=2) + '\n')
    try:
        need(sha(Path(__file__)) == job['runner_sha256'], 'Local runner changed')
        old_job = ROOT / 'docs/parity/p02/admission/JOB.json'
        need(sha(old_job) == job['preserved_original_job_sha256'], 'Historical job changed')
        original = json.loads(old_job.read_text())
        spec = importlib.util.spec_from_file_location('preserved_p02_gate', ROOT/'scripts/run-p02-regressions.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.source_check(original)
        need(job['commands'] == original['commands'] and job['env'] == original['env'], 'Execution scope changed')
        receipt['source_verified'] = True
        receipt['initial_preflight'] = local_preflight()
        if not args.execute:
            receipt['status'] = 'preflight_passed_no_execution'
            return 0
        need(not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True), 'Dirty candidate')
        output_root = Path('/workspace/build/builderbob/mbse-craft/p02-readiness')
        output_root.mkdir(parents=True, exist_ok=True)
        with (output_root/'local-target.lock').open('a') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            need(not (output_root/('job-' + sha(old_job) + '.started')).exists(), 'Original job already spent')
            marker = output_root/('local-job-' + sha(JOB) + '.started')
            need(not marker.exists(), 'Local job already spent; freeze only unexecuted remainder')
            output = output_root/('local-attempt-' + dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ'))
            output.mkdir(exist_ok=False)
            receipt.update(output_directory=str(output), checkout_commit=subprocess.check_output(
                ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip())
            (output/'LOCAL_JOB.json').write_bytes(JOB.read_bytes())
            env = os.environ.copy()
            env.update(job['env'])
            for key in ('TMPDIR', 'CARGO_TARGET_DIR'):
                destination = Path(env[key])
                need(destination.is_relative_to(output_root), 'Output outside allocated target')
                destination.mkdir(parents=True, exist_ok=True)
            for index, command in enumerate(job['commands']):
                module.source_check(original)
                preflight = local_preflight()
                log_path = output/f'{index:02}.log'
                entry = dict(index=index, argv=command, started_utc=now(), preflight=preflight, log=str(log_path))
                receipt['commands'].append(entry)
                if index == 0:
                    with marker.open('x') as stream:
                        stream.write(json.dumps(dict(receipt=str(record_path), output=str(output))) + '\n')
                with log_path.open('xb') as stream:
                    child = subprocess.Popen(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
                    entry['pid'] = child.pid
                    try:
                        entry['start_ticks'] = int(Path(f'/proc/{child.pid}/stat').read_text().rsplit(')', 1)[1].split()[19])
                    except FileNotFoundError:
                        entry['start_ticks'] = 'exited_before_identity_read'
                    receipt.update(status='running', owned_jobs=[dict(pid=child.pid, start_ticks=entry['start_ticks'])])
                    save()
                    entry['exit_code'] = child.wait()
                receipt['owned_jobs'] = []
                entry.update(ended_utc=now(), log_sha256=sha(log_path),
                             passed_tests=re.findall(r'^test ([^\n]+) \.\.\. ok$', log_path.read_text(errors='replace'), re.M))
                save()
                if entry['exit_code']:
                    receipt['status'] = 'failed'
                    return 1
            module.source_check(original)
            binary = Path(env['CARGO_TARGET_DIR'])/'debug/mbse-desktop'
            preserved = output/'mbse-desktop'
            with binary.open('rb') as src, preserved.open('xb') as dest:
                shutil.copyfileobj(src, dest)
            preserved.chmod(0o700)
            receipt.update(status='regressions_and_build_passed_native_not_run',
                           binary_sha256=sha(preserved), binary_bytes=preserved.stat().st_size,
                           preserved_binary=str(preserved))
            return 0
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        receipt.update(status='held_or_failed', reason=str(error))
        return 2
    finally:
        receipt['ended_utc'] = now()
        save()
        print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    sys.exit(main())
