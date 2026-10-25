#!/usr/bin/env python3
"""Actual blank-author M1-AUTHOR input, saved-byte checks and normal reopen.

Uses the retained private XTest driver. Never imports or seeds a model fixture.
Receipts distinguish attempted actions, observed app exits and checked bytes.
"""
import argparse
import datetime as dt
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat()


class Native:
    def __init__(self, exe, run, extra, evidence):
        self.run, self.evidence = run, evidence
        self.stderr = run.parent/(run.name + '-driver.log')
        self.child = subprocess.Popen([sys.executable, str(ROOT/'scripts/verify-native.py'),
            '--exe', str(exe), '--run-dir', str(run), '--', *extra],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr.open('x'), text=True, bufsize=1)
        self.ready = self.read()
        assert self.ready.get('ready'), self.ready
        self.identities = {}
        for key in ('app_pid', 'server_pid'):
            pid = self.ready[key]
            self.identities[key] = dict(pid=pid, start_ticks=int(Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[19]))
        evidence.append(dict(run=str(run), started_utc=utc(), ready=self.ready, identities=self.identities, argv_extra=extra))

    def read(self):
        assert select.select([self.child.stdout], [], [], 40)[0], 'Native driver response timeout'
        line = self.child.stdout.readline()
        assert line, self.stderr.read_text()
        return json.loads(line)

    def send(self, **command):
        self.child.stdin.write(json.dumps(command) + '\n')
        self.child.stdin.flush()
        return self.read()

    def audit(self):
        return json.loads((self.run/'state/ui-geometry.json').read_text())

    def seek(self, widget):
        for _ in range(16):
            audit = self.audit()
            rect = audit['widgets'].get(widget)
            if rect and rect[2] > rect[0] and rect[3] > rect[1] and 0 <= rect[1] < rect[3] <= 1000:
                return
            assert rect is not None, 'Missing live widget: ' + widget
            popup = '-option:' in widget
            self.send(op='scroll', x=int((rect[0]+rect[2])/2) if popup else 160,
                      y=int(min(rect[1], rect[3])-16) if popup else 650,
                      steps=-4 if rect[3] < 200 else 4, pause=.25)
        raise AssertionError('Cannot reveal live widget: ' + widget)

    def click(self, widget):
        self.seek(widget)
        return self.send(op='click', widget=widget)

    def fill(self, widget, text):
        self.click(widget)
        self.send(op='key', key='a', mods=['Control_L'])
        self.send(op='text', text=text)

    def capture(self, name):
        event = self.send(op='capture', name=name)
        (self.run/(name+'.ui.json')).write_text(json.dumps(self.audit(), indent=2)+'\n')
        return event

    def save(self, name):
        target = self.run/name
        assert not target.exists()
        self.fill('save-path', str(target))
        self.click('action:Save')
        for _ in range(40):
            audit = self.audit()
            if audit['save_status'] == 'Saved' and target.exists():
                return json.loads(target.read_text())['snapshot']
            if audit['error']:
                raise AssertionError(audit['feedback'])
            time.sleep(.1)
        raise AssertionError('Native save did not complete')

    def close(self):
        result = self.send(op='close', pause=1)
        assert result['app_returncode'] == 0, result
        self.child.stdin.write('{"op":"stop"}\n')
        self.child.stdin.flush()
        self.child.stdin.close()
        assert self.child.wait(timeout=15) == 0, self.stderr.read_text()
        self.evidence[-1].update(normal_app_exit=0, driver_exit=0, ended_utc=utc())
        for identity in self.identities.values():
            path = Path(f"/proc/{identity['pid']}/stat")
            assert not path.exists() or int(path.read_text().rsplit(')', 1)[1].split()[19]) != identity['start_ticks'], 'Owned process remains'
        self.evidence[-1]['owned_processes_absent'] = True


def check_graph(snapshot):
    records = snapshot['records']
    assert list(records) == ['1', '1000', '1001', '1002', '1003']
    assert [records[key]['kind'] for key in records] == ['System', 'Mission', 'Capability', 'Objective', 'Measure']
    assert records['1']['fields']['purpose'] == 'Preserve typed engineering decisions'
    assert records['1001']['fields']['contributesTo'] == [1000]
    assert records['1002']['fields']['subject'] == {'id':1001, 'kind':'Capability'}
    measure = records['1003']['fields']
    assert measure['forObjective'] == 1002 and measure['criterion'] == 95
    assert any(row['key']['record']==1003 and row['key']['field']=='criterion' and row['text']=='9e' for row in snapshot['drafts'])
    return measure


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--expected-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert args.output.is_absolute() and str(args.output).startswith('/workspace/build/builderbob/mbse-craft/p02-readiness/')
    assert digest(args.exe) == args.expected_sha256
    args.output.mkdir(exist_ok=False)
    receipt = dict(schema='mbse-craft.p02-native-acceptance/1', started_utc=utc(), status='running',
        binary=str(args.exe), binary_sha256=digest(args.exe), action_script_sha256=digest(Path(__file__)),
        driver_sha256=digest(ROOT/'scripts/verify-native.py'), runs=[], original_observer_admission_claimed=False)
    spec = importlib.util.spec_from_file_location('local_p02', ROOT/'scripts/run-p02-local.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    native = None
    try:
        with (args.output.parent/'local-target.lock').open('a') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            receipt['preflight_blank'] = module.local_preflight()
            native = Native(args.exe, args.output/'blank', [], receipt['runs'])
            native.capture('00-blank')
            native.click('tree:1')
            native.fill('field:1:name', 'P02 engineering system')
            native.fill('field:1:purpose', 'Preserve typed engineering decisions')
            for kind, name in [('Mission','Assess mission outcomes'), ('Capability','Measure operational capability'), ('Objective','Keep engineering evidence reviewable')]:
                native.click('action:Add '+kind)
                native.fill('creation:name', name)
                native.click('creation:commit')
            native.click('tree:1001')
            native.click('mission-link:1001:1000')
            native.click('action:Undo')
            assert 'Model history applied' in native.audit()['feedback']
            native.capture('01-model-undo')
            native.click('action:Redo')
            assert 'Model history applied' in native.audit()['feedback']
            native.click('tree:1002')
            native.click('subject:1002')
            native.click('subject-option:1002:1001')
            native.click('action:Add Measure')
            native.fill('creation:name', 'Retained distance and speed analysis')
            native.click('creation:commit')
            native.click('tree:1003')
            native.click('measure-unit:1003')
            native.click('measure-unit-option:1003:metre')
            native.fill('field:1003:criterion', '95')
            native.fill('field:1003:criterion', '9e')
            native.send(op='key', key='z', mods=['Control_L'])
            assert 'Field text history applied' in native.audit()['feedback']
            native.capture('02-field-undo')
            native.send(op='key', key='z', mods=['Control_L','Shift_L'])
            assert 'Field text history applied' in native.audit()['feedback']
            native.capture('03-field-redo')
            native.click('calc:Add calculation')
            native.fill('field:1003:calc.input.x', '120')
            native.click('calc:Add duration input')
            native.fill('field:1003:calc.input.x1', '8')
            native.fill('field:1003:calc.expression', 'x / x1')
            native.click('output-unit:1003')
            native.click('output-unit-option:1003:MetrePerSecond')
            native.seek('field:1003:calc.expression')
            native.capture('04-speed-15-metres-per-second')
            speed = native.save('speed.kernspan')
            measure = check_graph(speed)
            calculation = measure['calculation']
            assert calculation['output'] == {'kind':'Speed','unit':'MetrePerSecond'}
            assert calculation['bindings']['x']['Literal']['decimal']=='120'
            assert calculation['bindings']['x1']['Literal']['decimal']=='8'
            receipt['speed_saved_definition_checked'] = True
            native.fill('field:1003:calc.expression', 'x')
            native.fill('field:1003:calc.input.x', '2500')
            native.click('output-unit:1003')
            native.click('output-unit-option:1003:Kilometre')
            native.seek('field:1003:calc.expression')
            native.capture('05-distance-2point5-kilometres')
            before = native.audit()['snapshot_hash']
            native.fill('save-path', str(native.run/'missing-parent/failure.kernspan'))
            native.click('action:Save')
            for _ in range(30):
                if native.audit()['error'] and not native.audit()['write_pending']:
                    break
                time.sleep(.1)
            failed = native.audit()
            assert failed['error'] and failed['snapshot_hash'] == before and failed['save_status'] != 'Saved', failed
            native.capture('06-controlled-save-failure')
            native.send(op='close')
            native.click('preserve:Cancel')
            assert native.audit()['snapshot_hash'] == before
            native.capture('07-cancel-retains-work')
            final = native.save('blank-authored.kernspan')
            measure = check_graph(final)
            assert measure['calculation']['output'] == {'kind':'Length','unit':'Kilometre'}
            assert measure['calculation']['bindings']['x']['Literal']['decimal'] == '2500'
            native.capture('08-saved-blank-authored')
            native.close()
            saved = args.output/'blank/blank-authored.kernspan'
            receipt['preflight_reopen'] = module.local_preflight()
            native = Native(args.exe, args.output/'reopen', ['--open',str(saved),'--select','1003'], receipt['runs'])
            native.seek('field:1003:criterion')
            native.capture('09-reopened-raw-draft')
            reopened = native.save('reopened.kernspan')
            assert reopened == final, 'Native snapshot changed across normal close/reopen'
            native.seek('field:1003:calc.expression')
            native.capture('10-reopened-calculation')
            native.close()
            native = None
            receipt.update(status='scripted_checks_passed_pixels_review_pending', saved_snapshot_sha256=hashlib.sha256(
                json.dumps(final,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                saved_file_sha256=digest(saved), snapshot_equal_after_reopen=True,
                limits=['Actual captured pixels require review before numeric UI result claims.',
                        'Same-agent workflow; independent reviewer acceptance remains separate.'])
            return 0
    except Exception as error:
        receipt.update(status='failed_or_partial', error=repr(error))
        if native and native.child.poll() is None:
            native.child.stdin.write('{"op":"stop"}\n')
            native.child.stdin.flush()
            native.child.wait(timeout=15)
            receipt['failure_cleanup'] = 'Driver stopped; no normal app-exit claim for interrupted run.'
        return 1
    finally:
        receipt['ended_utc'] = utc()
        (args.output/'RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps(receipt,indent=2))


if __name__ == '__main__':
    sys.exit(main())
