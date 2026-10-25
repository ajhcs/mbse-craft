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
    def __init__(self,exe,run,extra,evidence):
        self.run,self.evidence=run,evidence
        self.stderr=run.parent/(run.name+'-driver.log')
        self.child=subprocess.Popen([sys.executable,str(ROOT/'scripts/verify-m1-native.py'),
            '--exe',str(exe),'--run-dir',str(run),'--',*extra],stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,stderr=self.stderr.open('x'),text=True,bufsize=1)
        self.ready=self.read();assert self.ready.get('ready'),self.ready
        self.identities={}
        for key in ('app_pid','server_pid'):
            pid=self.ready[key];self.identities[key]=dict(pid=pid,start_ticks=int(Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19]))
        evidence.append(dict(run=str(run),started_utc=base.utc(),ready=self.ready,identities=self.identities,argv_extra=extra))

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


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--job',type=Path,required=True);args=parser.parse_args()
    job=json.loads(args.job.read_text());exe=Path(job['binary']);assert base.digest(exe)==job['binary_sha256']
    for relative,expected in job['script_hashes'].items():assert base.digest(ROOT/relative)==expected,relative
    for name,expected in job['inputs'].items():assert base.digest(Path(name))==expected,name
    lease=Path('/workspace/build/builderbob/mbse-craft/milestone1/target.lock').open('a')
    fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
    output=Path(job['output']);assert str(output).startswith('/workspace/build/builderbob/mbse-craft/milestone1/');output.mkdir(exist_ok=False)
    check_spec=importlib.util.spec_from_file_location('local_preflight',ROOT/'scripts/run-p02-local.py');check=importlib.util.module_from_spec(check_spec);check_spec.loader.exec_module(check)
    receipt=dict(schema='mbse-craft.m1-native-session/1',started_utc=base.utc(),job_sha256=base.digest(args.job),status='active',runs=[],preflight=check.local_preflight())
    n=NativeM1(exe,output/'run',job['argv_extra'],receipt['runs'])
    context=dict(n=n,output=output,receipt=receipt,Path=Path,json=json,base=base,check=check)
    actions=(output/'CONTROLLER_ACTIONS.py').open('x')
    try:
        print(json.dumps(dict(ready=True,output=str(output),selected=n.audit()['selected'],widget_count=len(n.audit()['widgets']))),flush=True)
        for line in sys.stdin:
            actions.write(line);actions.flush()
            if line.strip()=='FINISH':
                assert n.child.poll()==0,'Close app normally and stop driver first'
                receipt['status']='actions_completed_review_pending';break
            try:
                exec(compile(line,'<recorded-acceptance-action>','exec'),context)
                print(json.dumps(dict(action_completed=True,app_running=n.child.poll() is None)),flush=True)
            except Exception as error:
                receipt.setdefault('action_errors',[]).append(dict(action=line.rstrip(),error=repr(error)))
                print(json.dumps(dict(action_completed=False,error=repr(error))),flush=True)
    finally:
        if n.child.poll() is None:
            n.child.stdin.write('{"op":"stop"}\n');n.child.stdin.flush();n.child.wait(timeout=15)
            receipt['interrupted_cleanup']='Driver stopped; normal app exit not claimed.'
        receipt['ended_utc']=base.utc();actions.close();(output/'RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n')
        lease.close()


if __name__=='__main__':main()
