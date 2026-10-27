import datetime,hashlib,json,os,pathlib,re,shutil,subprocess,sys
root=pathlib.Path('/workspace/build/builderbob/mbse-craft/views-origin-paired-save')
job_path=root/'JOB.json';expected='2526d86ff57bde75046b1bd676dba18975ce13f5065fe959f503884593db8e1f'
owner='01a1653d-9c16-76a3-ad80-df98ba1e5c94'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc)
def date(s):return datetime.datetime.fromisoformat(s.replace('Z','+00:00'))
def stamp():return now().isoformat()
assert sha(job_path)==expected,'Job changed';job=json.loads(job_path.read_text());cwd=pathlib.Path(job['cwd']);manifest_path=pathlib.Path(job['source_manifest']);assert sha(manifest_path)==job['source_manifest_sha256'];manifest=json.loads(manifest_path.read_text())
def source_check():
 for f in manifest['files']:
  p=cwd/f['path'];assert p.stat().st_size==f['bytes'] and sha(p)==f['sha256'],'Source changed: '+f['path']
 assert sha(cwd/'Cargo.lock')==job['cargo_lock_sha256']
def admission():
 a=json.loads((root/'ADMISSION.json').read_text());h=a['health'];w=a['watchdog'];current=now()
 assert a['schema']=='builderbob.job-bound-health/1' and a['owner_thread_id']==owner
 assert a['expected_job_manifest_sha256']==expected and a['resource_admission']=='PASS'
 assert date(a['checked_utc'])<=current<date(a['expires_utc']),'Expired/unknown resource admission'
 assert a['watchdog_identity_matches'] is True and h['monitor_status']=='OK' and h['observer_state']=='RUNNING'
 assert h['exact_process_start_matches'] is True and h['status_pid_matches'] is True and h['trend_matches_latest_observation'] is True
 assert w['decision']['level']=='OK' and w['decision']['hold_new_launches'] is False and w['notification_hold'] is False
 assert 0<=(current-date(h['latest_observation']['utc'])).total_seconds()<=120
 assert 0<=(current-date(w['checked_utc'])).total_seconds()<=120
 assert h['heartbeat_age_seconds']+(current-date(h['checked_utc'])).total_seconds()<=120
 i=h['latest_observation']['identity'];assert i['pid']==757634 and i['start_ticks']==8481870 and i['uid']==1000 and i['boot_id']=='b0688b87-f383-4de6-8124-071099ae1547' and i['pid_namespace']=='pid:[4026531836]' and i['exe']=='/home/builderbob/.codex/packages/standalone/releases/0.163.0-x86_64-unknown-linux-musl/bin/codex'
 return a
source_check()
# Compiler exclusivity is the parent's explicit single-job allocation and
# completed prior-wave/no-new-launch evidence, not a nonexistent Linux mutex.
allocation=json.loads((root/'ALLOCATION.json').read_text());assert allocation['owner_thread_id']==owner and allocation['compiler_allocated'] is True
prior=pathlib.Path(allocation['prior_completed_receipt']);assert sha(prior)==allocation['prior_completed_receipt_sha256']
prior_record=json.loads(prior.read_text());assert prior_record['status']=='Passed' and all(c['exit_code']==0 for c in prior_record['commands'])
assert allocation['new_cargo_launches_this_resume_before_gate']==0
previous=root/'attempt-20261023T120544518078Z/RECEIPT.json'
previous_sha='71f0d414150d83365049d4b71f74e7f191ae4f16d2eba650aca9b87c35f79c83'
assert sha(previous)==previous_sha
completed=json.loads(previous.read_text());assert completed['status']=='HeldOrRefused' and completed['job_manifest_sha256']==expected and completed['source_sha256']==manifest['source_sha256']
assert len(completed['commands'])==2 and all(c['exit_code']==0 and sha(pathlib.Path(c['log']))==c['log_sha256'] for c in completed['commands'])
expected_tests={pathlib.Path(p).stem+'::tests::'+name for p,names in job['expected_tests_unrun'].items() for name in names}
actual_tests={name for c in completed['commands'] for name in c['actual_passed_tests']};assert actual_tests==expected_tests and len(actual_tests)==38
first=admission();out=root/('attempt-'+now().strftime('%Y%m%dT%H%M%S%fZ'));out.mkdir()
for name in ['JOB.json','SOURCE_MANIFEST.json','ALLOCATION.json']:shutil.copy2(root/name,out/name)
record={'profile':'mbse-craft.preservation-fix-retest/1','status':'Running','started_utc':stamp(),'job_manifest_sha256':expected,'source_sha256':manifest['source_sha256'],'source_manifest_sha256':sha(manifest_path),'execution_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=cwd,text=True).strip(),'commands':[],'continued_from':{'receipt':str(previous),'sha256':previous_sha,'passed_tests':sorted(actual_tests),'owned_completed_children':[{'pid':c['pid'],'start_ticks':c['start_ticks'],'exit_code':c['exit_code']} for c in completed['commands']]},'allocation_evidence':allocation,'renderer':False,'uploads':False}
def save(): (out/'RECEIPT.json').write_text(json.dumps(record,indent=2)+'\n')
save();env=os.environ.copy();env.update(job['env'])
try:
 for index,command in [(2,job['commands'][2])]:
  source_check();packet=admission();log=out/('%02d.log'%index);entry={'argv':command,'started_utc':stamp(),'health_packet':packet,'log':str(log)}
  with log.open('wb') as stream:
   p=subprocess.Popen(command,cwd=cwd,env=env,stdout=stream,stderr=subprocess.STDOUT);entry['pid']=p.pid
   try:entry['start_ticks']=int(pathlib.Path('/proc/%s/stat'%p.pid).read_text().rsplit(')',1)[1].split()[19])
   except FileNotFoundError:entry['start_ticks']='Exited before owned-PID observation'
   entry['exit_code']=p.wait()
  entry.update(ended_utc=stamp(),log_sha256=sha(log),actual_passed_tests=re.findall(r'^test ([^\n]+) \.\.\. ok$',log.read_text(),re.M));record['commands'].append(entry);save()
  print(json.dumps({'command':index,'pid':p.pid,'exit_code':entry['exit_code'],'log':str(log),'receipt':str(out/'RECEIPT.json')}),flush=True)
  if entry['exit_code']!=0:record['status']='Failed';break
 else:
  binary=pathlib.Path(job['env']['CARGO_TARGET_DIR'])/'debug/mbse-desktop';dest=out/'mbse-desktop';shutil.copy2(binary,dest);record.update(status='Passed',binary_path=str(dest),binary_sha256=sha(dest),binary_bytes=dest.stat().st_size)
 source_check();record['source_unchanged']=True
except Exception as error:
 record.update(status='HeldOrRefused',reason=str(error));raise
finally:
 record['ended_utc']=stamp();save();print(json.dumps({'status':record['status'],'receipt':str(out/'RECEIPT.json'),'binary_sha256':record.get('binary_sha256')}),flush=True)
sys.exit(0 if record['status']=='Passed' else 1)
