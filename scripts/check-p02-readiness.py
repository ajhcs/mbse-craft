#!/usr/bin/env python3
"""Audit retained PR 1 evidence without compiling or running the application."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PR1 = '5b907c333d423df60de7d539da2a0900941825aa'
EPOCH = 'docs/continuation-m2/verification/epoch-217c977c-source-edit'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def checked(condition, message):
    if not condition:
        raise ValueError(message)


def original(path):
    """Use the reviewed PR 1 commit, never editable local receipt declarations."""
    return subprocess.check_output(['git', '-C', str(ROOT), 'show', f'{PR1}:{path}'])


def old_json(path):
    return json.loads(original(path))


def build_inputs():
    paths = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z']).decode().split('\0')
    selected = sorted(p for p in paths if p and
                      (p in ('Cargo.toml', 'Cargo.lock') or p.startswith(('crates/', 'vendor/'))))
    result = []
    combined = hashlib.sha256()
    for path in selected:
        raw = (ROOT / path).read_bytes()
        encoded = path.encode()
        combined.update(len(encoded).to_bytes(8, 'little'))
        combined.update(encoded)
        combined.update(len(raw).to_bytes(8, 'little'))
        combined.update(raw)
        result.append(dict(path=path, bytes=len(raw), sha256=digest(raw)))
    return combined.hexdigest(), result


def tests_in(raw):
    return re.findall(r'^test ([^\n]+) \.\.\. ok$', raw.decode(), re.MULTILINE)


def audit(verify_binary=False):
    manifest = old_json(f'{EPOCH}/SOURCE_MANIFEST.json')
    source_hash, current = build_inputs()
    checked(current == manifest['files'] and source_hash == manifest['source_sha256'],
            'Current build inputs differ from the retained final source-edit epoch')
    epoch = old_json(f'{EPOCH}/EPOCH.json')
    checked(epoch['source_sha256'] == source_hash, 'Epoch/source binding differs')
    ui = old_json(f'{EPOCH}/UI_TEST_BUILD_RECEIPT.json')
    core = old_json(f'{EPOCH}/CORE_TEST_RECEIPT.json')
    native = old_json(f'{EPOCH}/NATIVE_RECEIPT.json')
    archive_manifest = old_json(f'{EPOCH}/LOG_ARCHIVES.json')
    logs, archive_results = {}, []
    for entry in archive_manifest['archives']:
        packed = original(f'{EPOCH}/{entry["archive"]}')
        raw = gzip.decompress(packed)
        checked(digest(packed) == entry['archive_sha256'] and digest(raw) == entry['original_sha256']
                and len(raw) == entry['original_bytes'], f'Log binding differs: {entry["archive"]}')
        logs[entry['original_name']] = raw
        archive_results.append(dict(archive=entry['archive'], archive_sha256=digest(packed),
                                    original_sha256=digest(raw), original_bytes=len(raw)))
    core_names, ui_names = tests_in(logs['core-tests.log']), tests_in(logs['ui-tests.log'])
    checked(len(core_names) == len(set(core_names)) == epoch['core_tests_passed'] == 50,
            'Historical core/desktop wave inventory differs')
    checked(len(ui_names) == len(set(ui_names)) == epoch['ui_tests_passed'] == 6,
            'Integrated UI wave inventory differs')
    checked(set(ui_names) == set(ui['expected_ui_tests']) == set(ui['actual_ui_tests_passed'])
            == set(epoch['ui_actual_test_names']), 'Integrated UI tests are not exactly the expected six')
    checked(digest(logs['core-tests.log']) == core['commands'][0]['log_sha256'], 'Core log/receipt mismatch')
    checked(digest(logs['ui-tests.log']) == ui['commands'][0]['log_sha256'], 'UI log/receipt mismatch')
    checked(digest(logs['ui-build.log']) == ui['commands'][1]['log_sha256'], 'Build log/receipt mismatch')
    checked(all(c['exit_code'] == 0 for c in ui['commands']), 'Integrated UI/build did not succeed')
    checked(ui['cargo_lock_sha256_before'] == ui['cargo_lock_sha256_after'] == digest((ROOT/'Cargo.lock').read_bytes()),
            'Current lock differs from final source-edit build')
    checked(ui['binary_sha256'] == epoch['binary_sha256'] == native['binary_sha256'], 'Binary receipt pins differ')
    checked(native['native_model']['native_save_and_open_actions_observed'] is True
            and native['native_model']['snapshot_hash_distinct_count'] == 1, 'Retained native round-trip evidence differs')
    checked(native['driver_exit_code'] == 0 and native['forced_app_cleanup'] is False
            and native['xauthority_removed'] is True
            and all(c['proc_path_exists'] is False for c in native['cleanup']), 'Retained normal-close evidence differs')
    captures = []
    for capture in epoch['selected_capture_files']:
        raw = original(f'{EPOCH}/{capture["path"]}')
        checked(digest(raw) == capture['sha256'] and len(raw) == capture['bytes'], 'Capture pin differs')
        captures.append(dict(path=capture['path'], bytes=len(raw), sha256=digest(raw)))
    vendor_checks = []
    for directory in ('vendor/craft-libs', 'vendor/egui-fork', 'vendor/winit-fork'):
        provenance = old_json(f'{directory}/SOURCE.json')
        checked((ROOT/directory/'SOURCE.json').read_bytes() == original(f'{directory}/SOURCE.json'), 'Vendor declaration changed')
        for entry in provenance['files']:
            checked(digest((ROOT/directory/entry['path']).read_bytes()) == entry['sha256'],
                    f'Vendor input differs: {directory}/{entry["path"]}')
        licenses = ('LICENSE',) if directory == 'vendor/winit-fork' else ('LICENSE-MIT', 'LICENSE-APACHE')
        for license_name in licenses:
            checked((ROOT/directory/license_name).is_file(), f'Missing retained license: {directory}/{license_name}')
        vendor_checks.append(dict(path=directory, revision=provenance['revision'], checked_files=len(provenance['files'])))
    checked((ROOT/'licenses/kernspan/LICENSE-MIT').is_file(), 'Missing retained Kernspan license')
    native_model = old_json(f'{EPOCH}/native-independent.kernspan')
    native_records = native_model['snapshot']['records']
    # Actual newest native evidence exercised a blank independent document, not
    # the larger P02 blank-to-authored-system workflow. Keep that distinction.
    checked(len(native_records) == 1 and native_records['1']['kind'] == 'System',
            'Update the native coverage analysis: retained fixture inventory changed')
    binary = dict(status='not_rehashed', reason='Optional local artifact check; receipts are already bound above.')
    if verify_binary:
        path = Path(ui['preserved_binary'])
        checked(path.is_absolute() and path.is_relative_to('/workspace'), 'Unexpected preserved artifact path')
        h = hashlib.sha256()
        with path.open('rb') as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                h.update(chunk)
        checked(h.hexdigest() == epoch['binary_sha256'], 'Preserved executable no longer matches')
        binary = dict(status='sha256_verified_without_execution', sha256=h.hexdigest(), bytes=path.stat().st_size)
    return dict(schema='mbse-craft.p02-readiness-audit/1', reviewed_pr=1, reviewed_head=PR1,
        result='retained_evidence_consistent_acceptance_pending',
        source_closure=dict(source_sha256=source_hash, files=len(current), exact_historical_match=True,
                            manifest_path=f'{EPOCH}/SOURCE_MANIFEST.json'),
        lock_sha256=digest((ROOT/'Cargo.lock').read_bytes()),
        historical_waves=[dict(scope='core/desktop focused wave before final UI integration',
             tests=core_names, count=50, wrapper_status=core['status'], new_execution=False),
            dict(scope='six integrated source-name UI tests on final compiled inputs', tests=ui_names,
                 count=6, wrapper_status=ui['status'], new_execution=False)],
        log_bindings=archive_results, selected_capture_bindings=captures, vendor_bindings=vendor_checks,
        preserved_binary=binary,
        newest_native_coverage=dict(record_count=len(native_records), record_kinds=['System'],
            evidence='Source inspection/name-copy plus unchanged independent blank native save/reopen; no new blank-author system/calculation/draft acceptance.',
            native_receipt_path=f'{EPOCH}/NATIVE_RECEIPT.json'),
        pending_gates=[dict(id='P02-REGRESSION', status='not_run', detail='Focused native-model/draft/calculation/preservation regressions for the declared integrated P02 scenario.'),
            dict(id='P02-NATIVE', status='not_run', detail='M1-AUTHOR at this exact compiled closure: blank author, typed links, 15 m/s, 2.5 km, raw 9e/accepted 95, history, failed-save/Cancel and normal-close/reopen.'),
            dict(id='P02-ADMISSION', status='not_available_in_this_review', detail='No fresh original resource-health packet or exact-job compiler/renderer allocation was supplied. Historical admission is expired and is not reused.')],
        new_application_tests=0, new_gui_runs=0, build_executed=False, service_changes=False,
        declaration='This read-only evidence audit does not make P02 accepted, approve/merge PR1, or sum historical results into a new passing suite.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, help='Write a new report under this isolated worktree; refuse replacement')
    parser.add_argument('--verify-preserved-binary', action='store_true', help='Hash, never execute, the explicitly pinned workspace binary')
    args = parser.parse_args()
    try:
        report = audit(args.verify_preserved_binary)
        if args.output:
            path = args.output.resolve()
            checked(path.is_relative_to(ROOT), 'Report must stay in the isolated worktree')
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('x') as stream:
                json.dump(report, stream, indent=2)
                stream.write('\n')
        print(json.dumps(dict(result=report['result'], source_files=report['source_closure']['files'],
            source_sha256=report['source_closure']['source_sha256'], historical_test_waves=[50, 6],
            checked_vendor_files=sum(x['checked_files'] for x in report['vendor_bindings']),
            preserved_binary=report['preserved_binary']['status'], p02_acceptance='pending', new_product_execution=False), indent=2))
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as exc:
        print(f'P02 EVIDENCE AUDIT FAILED: {exc}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
