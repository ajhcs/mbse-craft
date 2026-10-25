import hashlib,json,os,pathlib,re
root=pathlib.Path('/workspace/build/builderbob/mbse-craft/views-origin-paired-save')
log=pathlib.Path('/home/builderbob/.codex/sessions/2026/10/22/rollout-2026-10-22T20-11-38-01a1653d-9c16-76a3-ad80-df98ba1e5c94.jsonl')
with log.open('rb') as f:
 f.seek(0,2);end=f.tell();f.seek(max(0,end-2*1024*1024));lines=f.read().decode().splitlines()
found=None
for line in reversed(lines):
 try:e=json.loads(line)
 except json.JSONDecodeError:continue
 p=e.get('payload',{})
 if e.get('type')!='response_item' or p.get('type')!='function_call_output' or p.get('name')!='send_message_to_thread':continue
 text=p.get('output','');text=text.get('text','') if isinstance(text,dict) else text
 if not isinstance(text,str):continue
 for match in re.finditer(r'\{\s*"schema"\s*:\s*"builderbob\.job-bound-health/1"',text):
  packet,length=json.JSONDecoder().raw_decode(text[match.start():])
  if packet.get('expected_job_manifest_sha256')=='2526d86ff57bde75046b1bd676dba18975ce13f5065fe959f503884593db8e1f' and packet.get('owner_thread_id')=='01a1653d-9c16-76a3-ad80-df98ba1e5c94':found=text[match.start():match.start()+length]
 if found:break
assert found is not None,'No actual incoming same-job receipt found'
prior=root/'ADMISSION.json'
if prior.exists():
 data=prior.read_bytes();archive=root/('ADMISSION-'+hashlib.sha256(data).hexdigest()+'.json')
 if not archive.exists():archive.write_bytes(data)
(root/'ADMISSION.pending.json').write_text(found+'\n');os.replace(root/'ADMISSION.pending.json',prior)
print(json.dumps({'staged_actual_incoming_receipt':True,'sha256':hashlib.sha256(prior.read_bytes()).hexdigest(),'checked_utc':json.loads(found)['checked_utc']}),flush=True)
