#!/usr/bin/env python3
"""P02 compiler gate. Preflight by default; genuine fresh admission is required.

No observer, resource sampler, service, renderer or automatic retry is created.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
JOB_PATH = ROOT / 'docs/parity/p02/admission/JOB.json'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def utc():
    return dt.datetime.now(dt.timezone.utc)


def date(value):
    parsed = dt.datetime.fromisoformat(value.replace('Z', '+00:00'))
    if parsed.tzinfo is None:
        raise ValueError('Admission time has no timezone')
    return parsed


def need(condition, reason):
    if not condition:
        raise ValueError(reason)


def load(path):
    return json.loads(path.read_text())


def source_check(job):
    need(sha(Path(__file__).read_bytes()) == job['runner_sha256'], 'Runner changed')
    need(Path(job['cwd']).resolve() == ROOT, 'Wrong isolated worktree')
    need(sha((ROOT / job['source_manifest']).read_bytes()) == job['source_manifest_sha256'], 'Source manifest changed')
    manifest = load(ROOT / job['source_manifest'])
    need(manifest['source_sha256'] == job['source_sha256'], 'Wrong build-input closure')
    names = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z']).decode().split('\0')
    current = sorted(p for p in names if p and
                     (p in ('Cargo.toml', 'Cargo.lock') or p.startswith(('crates/', 'vendor/'))))
    need(current == [r['path'] for r in manifest['files']], 'Build-input file set changed')
    for row in manifest['files']:
        raw = (ROOT / row['path']).read_bytes()
        need(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Build input changed: ' + row['path'])
    need(sha((ROOT / 'Cargo.lock').read_bytes()) == job['cargo_lock_sha256'], 'Lock changed')
    need(sha((ROOT / 'docs/parity/p02/EXECUTION_PLAN.json').read_bytes()) == job['original_execution_plan_sha256'], 'Prepared plan changed')
    need(job['env']['CARGO_BUILD_JOBS'] == job['env']['RUST_TEST_THREADS'] == '1', 'Single-job policy changed')


def admission(job, job_sha, admission_path, allocation_path):
    # Raw source packets must arrive through the existing supported coordinator.
    # This checks freshness/binding; it does not mint or authenticate their origin.
    packet = load(admission_path)
    allocation = load(allocation_path)
    current = utc()
    need(packet['schema'] == 'builderbob.job-bound-health/1', 'Wrong admission schema')
    need(packet['owner_thread_id'] == allocation['owner_thread_id'] == job['owner_thread_id'], 'Wrong owner')
    need(packet['expected_job_manifest_sha256'] == allocation['job_manifest_sha256'] == job_sha, 'Wrong exact job')
    need(packet['resource_admission'] == 'PASS', 'Resource admission is not PASS')
    need(date(packet['checked_utc']) <= current < date(packet['expires_utc']), 'Expired/future admission')
    need(packet['watchdog_identity_matches'] is True, 'Watchdog identity mismatch')
    health, watchdog = packet['health'], packet['watchdog']
    need(health['monitor_status'] == 'OK' and health['observer_state'] == 'RUNNING', 'Observer not OK/RUNNING')
    for key in ('exact_process_start_matches', 'status_pid_matches', 'trend_matches_latest_observation'):
        need(health[key] is True, 'Original observer evidence mismatch: ' + key)
    need(watchdog['decision']['level'] == 'OK' and watchdog['decision']['hold_new_launches'] is False
         and watchdog['notification_hold'] is False, 'Watchdog holds new launches')
    for label, stamp in [('observation', health['latest_observation']['utc']), ('watchdog', watchdog['checked_utc'])]:
        need(0 <= (current - date(stamp)).total_seconds() <= 120, label + ' freshness exceeds 120 seconds')
    need(0 <= health['heartbeat_age_seconds'] + (current - date(health['checked_utc'])).total_seconds() <= 120,
         'Heartbeat freshness exceeds 120 seconds')
    need(health['latest_observation']['identity'] == allocation['expected_appserver_identity'],
         'Allocation and original observer app-server identities differ')
    need(allocation['commands_authorized'] == job['commands'], 'Allocated commands differ')
    need(allocation['target_dir'] == job['env']['CARGO_TARGET_DIR'], 'Wrong allocated target')
    need(allocation['target_exclusive_confirmed'] is True and allocation['build_launch_authorized'] is True,
         'No exclusive compiler allocation')
    return packet, allocation


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--admission', type=Path, default=ROOT/'work/p02-admission/ADMISSION.json')
    ap.add_argument('--allocation', type=Path, default=ROOT/'work/p02-admission/ALLOCATION.json')
    ap.add_argument('--record', type=Path, required=True, help='New preflight record inside this isolated worktree')
    ap.add_argument('--execute', action='store_true', help='Run the frozen commands only after genuine admission; default only preflights')
    args = ap.parse_args()
    record_path = args.record.resolve()
    need(record_path.is_relative_to(ROOT) and not record_path.exists(), 'Record must be a new path in the isolated worktree')
    record_path.parent.mkdir(parents=True, exist_ok=True)
    job_raw = JOB_PATH.read_bytes()
    job, job_sha = json.loads(job_raw), sha(job_raw)
    record = dict(schema='mbse-craft.p02-regression-gate-receipt/1', started_utc=utc().isoformat(),
                  status='preflight', job_sha256=job_sha, runner_sha256=sha(Path(__file__).read_bytes()),
                  commands=[], owned_jobs=[], gui=False, new_monitor=False)
    def save():
        record_path.write_text(json.dumps(record, indent=2) + '\n')
    try:
        source_check(job)
        record['source_verified'] = True
        need(args.admission.is_file(), 'No original exact-job admission packet available')
        need(args.allocation.is_file(), 'No exact-job compiler/target allocation available')
        packet, allocation = admission(job, job_sha, args.admission, args.allocation)
        record['admission_verified'] = True
        if not args.execute:
            record['status'] = 'preflight_passed_no_execution'
            return 0
        need(not subprocess.check_output(['git', '-C', str(ROOT), 'status', '--porcelain'], text=True),
             'Candidate has dirty/untracked files')
        out_root = Path('/workspace/build/builderbob/mbse-craft/p02-readiness')
        need(shutil.disk_usage(ROOT).free >= 8 * 1024**3, 'Less than 8 GiB workspace reserve for the new target')
        spent = out_root / ('job-' + job_sha + '.started')
        need(not spent.exists(), 'This job already started; preserve results and freeze only an unexecuted remainder')
        out = out_root / ('attempt-' + utc().strftime('%Y%m%dT%H%M%S%fZ'))
        out.mkdir(parents=True, exist_ok=False)
        (out/'JOB.json').write_bytes(job_raw)
        for key in ('CARGO_TARGET_DIR', 'TMPDIR'):
            path = Path(job['env'][key])
            need(path.is_relative_to(out_root), 'Output outside dedicated target/temp root')
            path.mkdir(parents=True, exist_ok=True)
        record.update(status='running', output_directory=str(out),
                      execution_checkout_commit=subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip())
        env = os.environ.copy()
        env.update(job['env'])
        save()
        for index, command in enumerate(job['commands']):
            source_check(job)
            packet, allocation = admission(job, job_sha, args.admission, args.allocation)
            (out/f'{index:02}-ADMISSION.json').write_bytes(args.admission.read_bytes())
            (out/f'{index:02}-ALLOCATION.json').write_bytes(args.allocation.read_bytes())
            log_path = out/f'{index:02}.log'
            entry = dict(index=index, argv=command, started_utc=utc().isoformat(), admission_sha256=sha(args.admission.read_bytes()),
                         allocation_sha256=sha(args.allocation.read_bytes()), log=str(log_path))
            record['commands'].append(entry)
            with log_path.open('xb') as stream:
                if index == 0:
                    with spent.open('x') as marker:
                        marker.write(json.dumps(dict(job_sha256=job_sha, receipt=str(record_path), output=str(out))) + '\n')
                child = subprocess.Popen(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
                entry['pid'] = child.pid
                try:
                    entry['start_ticks'] = int(Path(f'/proc/{child.pid}/stat').read_text().rsplit(')', 1)[1].split()[19])
                except FileNotFoundError:
                    entry['start_ticks'] = 'exited_before_owned_identity_read'
                record['owned_jobs'] = [dict(pid=child.pid, start_ticks=entry['start_ticks'])]
                save()
                entry['exit_code'] = child.wait()
            record['owned_jobs'] = []
            raw = log_path.read_bytes()
            entry.update(ended_utc=utc().isoformat(), log_sha256=sha(raw),
                         passed_tests=re.findall(r'^test ([^\n]+) \.\.\. ok$', raw.decode(errors='replace'), re.M))
            save()
            if entry['exit_code'] != 0:
                record['status'] = 'failed'
                return 1
        source_check(job)
        binary = Path(job['env']['CARGO_TARGET_DIR'])/'debug/mbse-desktop'
        preserved = out/'mbse-desktop'
        with binary.open('rb') as src, preserved.open('xb') as dest:
            shutil.copyfileobj(src, dest)
        preserved.chmod(0o700)
        record.update(status='regressions_and_build_passed_gui_not_run', binary_sha256=sha(preserved.read_bytes()),
                      binary_bytes=preserved.stat().st_size, preserved_binary=str(preserved))
        return 0
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        record.update(status='held_or_refused_before_next_child', reason=str(error))
        return 2
    finally:
        record['ended_utc'] = utc().isoformat()
        save()
        print(json.dumps(record, indent=2))


if __name__ == '__main__':
    sys.exit(main())
