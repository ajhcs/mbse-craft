import datetime,hashlib,importlib.util,json,os,pathlib,shutil,subprocess
BASE=pathlib.Path('/workspace/build/builderbob/mbse-craft/recovery-layout-preview-shortcut-fix/native-gate')
ROOT=pathlib.Path('/workspace/worktrees/builderbob/mbse-craft/recovery-layout-preview')
EXPECTED='07d14544a9795af70cd042c27b847356de731e76b7c1322cc3a0cc3b621a3222'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
job=json.loads((BASE/'JOB.json').read_text());assert sha(BASE/'JOB.json')==EXPECTED
route=json.loads((BASE/'ROUTE.json').read_text());assert route['native_job_sha256']==EXPECTED
assert sha(BASE/'driver.py')==route['driver_sha256']
spec=importlib.util.spec_from_file_location('owned_native_driver',BASE/'driver.py');driver=importlib.util.module_from_spec(spec);spec.loader.exec_module(driver)
a=json.loads((BASE/'ALLOCATION.json').read_text());assert a['renderer_allocated'] is True and a['native_launch_authorized'] is True and a['job_manifest_sha256']==EXPECTED and a['owner_thread_id']==job['owner_thread']
assert os.getuid()==1000
manifest=json.loads(pathlib.Path(job['source_manifest']).read_text());assert manifest['source_sha256']==job['source_sha256']
for row in manifest['files']:assert sha(ROOT/row['path'])==row['sha256'],'Source changed: '+row['path']
assert sha(ROOT/'Cargo.lock')==job['cargo_lock_sha256'] and sha(pathlib.Path(job['binary']))==job['binary_sha256']
prior=pathlib.Path(job['build_receipt']);assert sha(prior)==job['build_receipt_sha256']
r=json.loads(prior.read_text());assert r['status']=='Passed' and all(c['exit_code']==0 for c in r['commands'])
assert not (BASE/'LAUNCH_RECEIPT.json').exists(),'Never replay an owned renderer epoch'
run=pathlib.Path(route['run_dir']);assert not run.exists()
health=driver.fresh()
ws=subprocess.run(['/usr/local/sbin/bob-admin','workspace-status'],capture_output=True,text=True);assert ws.returncode==0 and json.loads(ws.stdout)['outcome']=='ok'
os.umask(0o077);copydir=BASE/'bin';assert not copydir.is_symlink();copydir.mkdir(exist_ok=True);exe=copydir/'mbse-desktop'
if not exe.exists():
 with pathlib.Path(job['binary']).open('rb') as source,exe.open('xb') as dest:shutil.copyfileobj(source,dest)
assert not exe.is_symlink() and sha(exe)==job['binary_sha256'];exe.chmod(0o700)
health=driver.fresh();env=os.environ.copy()
for key in ['DISPLAY','XAUTHORITY','MBSE_NATIVE_INNER']:env.pop(key,None)
command=['/usr/bin/python3','-B',str(BASE/'driver.py'),'--dpi','96','--exe',str(exe),'--run-dir',str(run),'--state-dir',str(BASE/'state'),'--config-dir',str(BASE/'config')]
record={'status':'STARTING_NOT_ACCEPTANCE','started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'job_sha256':EXPECTED,'job':job,'route':route,'launcher_sha256':sha(pathlib.Path(__file__)),'allocation':a,'health':health,'command':command,'launcher_identity':driver.identity(os.getpid()),'executable_copy':str(exe),'executable_copy_sha256':sha(exe),'compiler':False,'uploads':False}
def save():(BASE/'LAUNCH_RECEIPT.json').write_text(json.dumps(record,indent=2)+'\n')
save();p=None
try:
 p=subprocess.Popen(command,env=env);record.update(status='RUNNING_NOT_ACCEPTANCE',driver_identity=driver.identity(p.pid));save();print(json.dumps({'event':'owned_renderer_starting','driver_identity':record['driver_identity'],'receipt':str(BASE/'LAUNCH_RECEIPT.json')}),flush=True)
 record['driver_exit_code']=p.wait(timeout=1200)
except subprocess.TimeoutExpired:
 record['failure']='Owned driver1200second budget exhausted; forced stop is not acceptance';p.terminate();record['driver_exit_code']=p.wait(timeout=8)
finally:
 if p is not None and p.poll() is None:p.terminate();p.wait(timeout=8)
 record.update(status='FINISHED_NOT_AUTOMATIC_ACCEPTANCE',ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),executable_sha256_after=sha(exe))
 if 'driver_identity' in record:record['driver_after_wait']=driver.absence(record['driver_identity'])
 save()
print(json.dumps({'event':'owned_renderer_finished','exit_code':record.get('driver_exit_code'),'receipt':str(BASE/'LAUNCH_RECEIPT.json')}),flush=True)
raise SystemExit(record.get('driver_exit_code',1))
