"""Prepared build-only launcher. Requires explicit exact-job allocation and fresh health.
No global compiler mutex, monitoring subprocess, GUI, or automatic retry.
"""
import datetime, hashlib, json, os, pathlib, re, shutil, subprocess, sys
root = pathlib.Path('/workspace/build/builderbob/mbse-craft/layout-periodic-recovery-r02')
expected = '16c649e8f8e85799b3ba45d9699887cf6d0437ef4b1f740e98c02da0001a3a06'
owner = '01a1653d-9c16-76a3-ad80-df98ba1e5c94'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def now(): return datetime.datetime.now(datetime.timezone.utc)
def date(s): return datetime.datetime.fromisoformat(s.replace('Z', '+00:00'))
def stamp(): return now().isoformat()
assert sha(root / 'JOB.json') == expected, 'Job changed'
job = json.loads((root / 'JOB.json').read_text())
cwd = pathlib.Path(job['cwd'])
manifest_path = pathlib.Path(job['source_manifest'])
assert sha(manifest_path) == job['source_manifest_sha256'], 'Manifest changed'
manifest = json.loads(manifest_path.read_text())
def source_check():
    assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=cwd, text=True).strip() == job['source_commit'], 'Source commit changed'
    for f in manifest['files']:
        p = cwd / f['path']
        assert p.stat().st_size == f['bytes'] and sha(p) == f['sha256'], 'Source changed: ' + f['path']
    assert sha(cwd / 'Cargo.lock') == job['cargo_lock_sha256']
def admission():
    a = json.loads((root / 'ADMISSION.json').read_text())
    h, w, current = a['health'], a['watchdog'], now()
    assert a['schema'] == 'builderbob.job-bound-health/1' and a['owner_thread_id'] == owner
    assert a['expected_job_manifest_sha256'] == expected and a['resource_admission'] == 'PASS'
    assert date(a['checked_utc']) <= current < date(a['expires_utc']), 'Expired admission'
    assert a['watchdog_identity_matches'] is True
    assert h['monitor_status'] == 'OK' and h['observer_state'] == 'RUNNING'
    assert h['exact_process_start_matches'] is True and h['status_pid_matches'] is True and h['trend_matches_latest_observation'] is True
    assert w['decision']['level'] == 'OK' and w['decision']['hold_new_launches'] is False and w['notification_hold'] is False
    assert 0 <= (current - date(h['latest_observation']['utc'])).total_seconds() <= 120
    assert 0 <= (current - date(w['checked_utc'])).total_seconds() <= 120
    assert h['heartbeat_age_seconds'] + (current - date(h['checked_utc'])).total_seconds() <= 120
    i = h['latest_observation']['identity']
    assert i == allocation['expected_appserver_identity'], 'Admission/independently allocated host identity differs'
    return a
source_check()
allocation = json.loads((root / 'ALLOCATION.json').read_text())
assert allocation['owner_thread_id'] == owner and allocation['job_manifest_sha256'] == expected
assert allocation['commands_authorized'] == job['commands']
assert allocation['target_dir'] == job['env']['CARGO_TARGET_DIR']
assert allocation['target_exclusive_confirmed'] is True
assert allocation['build_launch_authorized'] is True
assert shutil.disk_usage(root).free >= job['disk_budget']['minimum_free_bytes_before_launch'], 'Insufficient planned disk reserve'
admission()
out = root / ('attempt-' + now().strftime('%Y%m%dT%H%M%S%fZ'))
out.mkdir()
for name in ['JOB.json', 'SOURCE_MANIFEST.json', 'ALLOCATION.json']:
    shutil.copy2(root / name, out / name)
record = {'profile': 'mbse-craft.periodic-recovery-r02-test-build-receipt/1',
    'status': 'Running', 'started_utc': stamp(), 'job_manifest_sha256': expected,
    'source_commit': job['source_commit'], 'source_sha256': manifest['source_sha256'],
    'launcher_sha256': sha(pathlib.Path(__file__)), 'allocation': allocation,
    'commands': [], 'gui': False, 'uploads': False, 'owned_jobs': []}
def save(): (out / 'RECEIPT.json').write_text(json.dumps(record, indent=2) + '\n')
save()
env = os.environ.copy()
env.update(job['env'])
try:
    for index, command in enumerate(job['commands']):
        source_check()
        health = admission()
        log = out / ('%02d.log' % index)
        entry = {'argv': command, 'started_utc': stamp(), 'health_packet': health, 'log': str(log)}
        with log.open('wb') as stream:
            child = subprocess.Popen(command, cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT)
            entry['pid'] = child.pid
            try:
                entry['start_ticks'] = int(pathlib.Path('/proc/%s/stat' % child.pid).read_text().rsplit(')', 1)[1].split()[19])
            except FileNotFoundError:
                entry['start_ticks'] = 'Exited before owned-PID observation'
            record['owned_jobs'] = [{'pid': child.pid, 'start_ticks': entry['start_ticks']}]
            save()
            entry['exit_code'] = child.wait()
        record['owned_jobs'] = []
        entry.update(ended_utc=stamp(), log_sha256=sha(log),
            actual_passed_tests=re.findall(r'^test ([^\n]+) \.\.\. ok$', log.read_text(), re.M))
        record['commands'].append(entry)
        save()
        print(json.dumps({'command': index, 'exit_code': entry['exit_code'], 'receipt': str(out / 'RECEIPT.json')}), flush=True)
        if entry['exit_code'] != 0:
            record['status'] = 'Failed'
            break
    else:
        binary = pathlib.Path(job['binary_source'])
        assert binary.stat().st_size <= job['disk_budget']['preserved_binary_max_bytes'], 'Binary exceeds preservation budget'
        dest = out / 'mbse-desktop'
        with binary.open('rb') as src, dest.open('xb') as target:
            shutil.copyfileobj(src, target)
        dest.chmod(0o700)
        assert sha(dest) == sha(binary), 'Preserved binary differs'
        record.update(status='Passed', binary_path=str(dest), binary_sha256=sha(dest), binary_bytes=dest.stat().st_size)
        record['workspace_free_bytes_after'] = shutil.disk_usage(root).free
    source_check()
    record['source_unchanged'] = True
except Exception as error:
    record.update(status='HeldOrRefused', reason=str(error))
    raise
finally:
    record['ended_utc'] = stamp()
    save()
    print(json.dumps({'status': record['status'], 'receipt': str(out / 'RECEIPT.json')}), flush=True)
sys.exit(0 if record['status'] == 'Passed' else 1)
