#!/usr/bin/env python3
"""Interactive acceptance controller; every submitted action and XTest event retained.

No commands run on import. The caller supplies a frozen binary/fixture job.
"""
import argparse
import datetime as dt
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('p02_native',ROOT/'scripts/run-p02-native.py')
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)


class NativeM1(base.Native):
    instances = []

    def __init__(self,exe,run,extra,evidence,config_seed=None):
        self.run,self.evidence=run,evidence
        self.stderr=run.parent/(run.name+'-driver.log')
        seed_args=['--config-seed',str(config_seed)] if config_seed is not None else []
        self.child=subprocess.Popen([sys.executable,str(ROOT/'scripts/verify-m1-native.py'),
            '--exe',str(exe),'--run-dir',str(run),*seed_args,'--',*extra],stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,stderr=self.stderr.open('x'),text=True,bufsize=1)
        self.identities={}
        self.record=dict(run=str(run),started_utc=base.utc(),identities=self.identities,argv_extra=extra)
        if config_seed is not None:self.record['config_seed']=dict(path=str(config_seed),sha256=base.digest(Path(config_seed)))
        evidence.append(self.record)
        self.instances.append(self)
        self.ready=self.read();assert self.ready.get('ready'),self.ready
        for key in ('app_pid','server_pid'):
            pid=self.ready[key];self.identities[key]=dict(pid=pid,start_ticks=int(Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19]))
        self.record['ready']=self.ready

    def close(self):
        result=self.send(op='close',pause=1)
        assert result['app_returncode']==0,result
        self.child.stdin.write('{"op":"stop"}\n');self.child.stdin.flush();self.child.stdin.close()
        assert self.child.wait(timeout=15)==0,self.stderr.read_text()
        self.record.update(normal_app_exit=0,driver_exit=0,ended_utc=base.utc())
        for identity in self.identities.values():
            path=Path(f"/proc/{identity['pid']}/stat")
            assert not path.exists() or int(path.read_text().rsplit(')',1)[1].split()[19])!=identity['start_ticks'],'Owned process remains'
        self.record['owned_processes_absent']=True

    def seek(self,widget):
        for _ in range(16):
            rect=self.audit()['widgets'].get(widget)
            assert rect is not None,'Missing live widget: '+widget
            if rect[2]>rect[0] and rect[3]>rect[1] and 0<=rect[1]<rect[3]<=1600:return
            popup='-option:' in widget
            self.send(op='scroll',x=int((rect[0]+rect[2])/2) if popup else 160,
                      y=int(min(rect[1],rect[3])-16) if popup else 650,
                      steps=-4 if rect[3]<200 else 4,pause=.25)
        raise AssertionError('Cannot reveal '+widget)

    def clear_adjacent_views_path(self):
        rect=self.audit()['widgets']['views:Open views']
        self.send(op='click',x=int(rect[0]-100),y=int((rect[1]+rect[3])/2))
        self.send(op='key',key='a',mods=['Control_L'])
        self.send(op='key',key='BackSpace')


def owned_drivers(receipt):
    return [driver for driver in NativeM1.instances if driver.evidence is receipt['runs']]


def running_runs(receipt):
    return [str(driver.run) for driver in owned_drivers(receipt) if driver.child.poll() is None]


def finish_runs(receipt):
    drivers=owned_drivers(receipt)
    assert drivers and len(drivers)==len(receipt['runs']),'Every recorded run must have an owned driver'
    for driver in drivers:
        assert driver.child.poll()==0,'Close every app normally and stop every driver first: '+str(driver.run)
        assert driver.record.get('normal_app_exit')==0 and driver.record.get('driver_exit')==0 and driver.record.get('owned_processes_absent') is True,'Normal cleanup not verified: '+str(driver.run)
    receipt['status']='actions_completed_review_pending'


def cleanup_runs(receipt):
    for driver in owned_drivers(receipt):
        if driver.child.poll() is None:
            result=dict(run=str(driver.run),normal_app_exit_claimed=False)
            try:
                driver.child.stdin.write('{"op":"stop"}\n');driver.child.stdin.flush()
                result['driver_exit']=driver.child.wait(timeout=15)
            except Exception as error:
                result['error']=repr(error)
            receipt.setdefault('interrupted_cleanup',[]).append(result)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--job',type=Path,required=True);args=parser.parse_args()
    job=json.loads(args.job.read_text());exe=Path(job['binary']);assert base.digest(exe)==job['binary_sha256']
    for relative,expected in job['script_hashes'].items():assert base.digest(ROOT/relative)==expected,relative
    for name,expected in job['inputs'].items():assert base.digest(Path(name))==expected,name
    lease=Path('/workspace/build/builderbob/mbse-craft/milestone1/target.lock').open('a')
    fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
    output=Path(job['output']);assert str(output).startswith('/workspace/build/builderbob/mbse-craft/milestone1/');output.mkdir(exist_ok=False)
    check_spec=importlib.util.spec_from_file_location('local_preflight',ROOT/'scripts/run-p02-local.py');check=importlib.util.module_from_spec(check_spec);check_spec.loader.exec_module(check)
    receipt=dict(schema='mbse-craft.m1-native-session/1',started_utc=base.utc(),job_sha256=base.digest(args.job),status='active',runs=[],preflight=check.local_preflight(compiler_roots=job.get('compiler_roots')))
    actions=None
    try:
        n=NativeM1(exe,output/'run',job['argv_extra'],receipt['runs'],job.get('config_seed'))
        context=dict(n=n,output=output,receipt=receipt,Path=Path,json=json,base=base,check=check)
        actions=(output/'CONTROLLER_ACTIONS.py').open('x')
        print(json.dumps(dict(ready=True,output=str(output),selected=n.audit()['selected'],widget_count=len(n.audit()['widgets']))),flush=True)
        for line in sys.stdin:
            actions.write(line);actions.flush()
            if line.strip()=='FINISH':
                finish_runs(receipt);break
            try:
                exec(compile(line,'<recorded-acceptance-action>','exec'),context)
                active=running_runs(receipt)
                print(json.dumps(dict(action_completed=True,app_running=bool(active),running_runs=active)),flush=True)
            except Exception as error:
                receipt.setdefault('action_errors',[]).append(dict(action=line.rstrip(),error=repr(error)))
                print(json.dumps(dict(action_completed=False,error=repr(error))),flush=True)
    finally:
        cleanup_runs(receipt)
        receipt['ended_utc']=base.utc()
        if actions is not None:actions.close()
        (output/'RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n')
        lease.close()


if __name__=='__main__':main()
