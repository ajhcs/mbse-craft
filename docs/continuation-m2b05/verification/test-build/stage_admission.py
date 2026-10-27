import datetime,hashlib,json,re
from pathlib import Path
r=Path('/workspace/build/builderbob/mbse-craft/native-find-usages');h='000e8e25a8b633609fb9b05e54d298e8e350f7b81487fcb892fbd449867067e4';j=json.loads((r/'JOB.json').read_text())
assert hashlib.sha256((r/'JOB.json').read_bytes()).hexdigest()==h
assert not list(r.glob('attempt-*')), 'Do not replay started gate'
p=Path('/home/builderbob/.codex/sessions/2026/10/22/rollout-2026-10-22T20-11-38-01a1653d-9c16-76a3-ad80-df98ba1e5c94.jsonl')
with p.open('rb') as f:
 f.seek(0,2);n=f.tell();f.seek(max(0,n-2500000));lines=f.read().decode().splitlines()
found=None
for line in reversed(lines):
 try:v=json.loads(line).get('payload',{});s=v.get('output','');s=s.get('text','') if isinstance(s,dict) else s
 except (ValueError,AttributeError):continue
 if v.get('type')!='function_call_output' or v.get('name')!='send_message_to_thread' or not isinstance(s,str):continue
 for m in re.finditer(r'\{\s*"schema"\s*:\s*"builderbob\.job-bound-health/1"',s):
  a,k=json.JSONDecoder().raw_decode(s[m.start():])
  if a.get('expected_job_manifest_sha256')==h:found=s[m.start():m.start()+k]
 if found:break
assert found, 'No original exact-job packet'
p=json.loads(found);now=datetime.datetime.now(datetime.timezone.utc)
assert p['resource_admission']=='PASS' and now<datetime.datetime.fromisoformat(p['expires_utc'])
a={'owner_thread_id':j['owner_thread'],'job_manifest_sha256':h,'commands_authorized':j['commands'],'target_dir':j['env']['CARGO_TARGET_DIR'],'target_exclusive_confirmed':True,'build_launch_authorized':True,'expected_appserver_identity':{'pid': 1072280, 'start_ticks': 14235850, 'uid': 1000, 'exe': '/home/builderbob/.codex/packages/standalone/releases/0.164.0-x86_64-unknown-linux-musl/bin/codex', 'pid_namespace': 'pid:[4026531836]', 'boot_id': 'b0688b87-f383-4de6-8124-071099ae1547'},'authority':'Existing authorized next scoped native navigation feature tests/build; fresh coordinator exact job evidence and normal action review','writer_confirmation':'Sole MBSE candidate/target writer; previous build/renderer identities absent; separate frozen old worktrees unchanged; no idle global reservation','gui':False,'uploads':False}
for name,s in [('ADMISSION.json',found+'\n'),('ALLOCATION.json',json.dumps(a,indent=2)+'\n')]:
 q=r/name;assert not q.exists();q.open('x').write(s)
print('Original inspection gate packet staged',now.isoformat(),flush=True)
