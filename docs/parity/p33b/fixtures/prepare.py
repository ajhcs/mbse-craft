"""Materialize predeclared P33a graphs for native execution; preparation is not execution/authoring acceptance."""
from pathlib import Path
import json,hashlib,copy
r=Path(__file__).resolve().parents[4];d=Path(__file__).resolve().parent
source=r/'docs/parity/p32c/verification/native-03/run/state-machine-final-qualified-copy.kernspan';side=source.with_suffix('.mbse-views.json');original=json.loads(source.read_text());views=json.loads(side.read_text());original_views=copy.deepcopy(views)
wire=lambda v:json.dumps(v,separators=(',',':'),ensure_ascii=False).encode();sha=lambda b:hashlib.sha256(b).hexdigest();assert sha(wire(original['snapshot']))==views['binding']['snapshot_sha256']
root=copy.deepcopy(original);s=root['snapshot'];machines=s['extensions']['namespaces']['mbse-craft.types/1']['state_machines'];oracle=json.load(open(r/'docs/parity/p33a/fixtures/state-traces.json'));mapping={};expected=[]
for index,(name,g)in enumerate(oracle['graphs'].items(),8):
 base=index*1000;remap=lambda old:old+base-1000;mapping[name]=base
 machines[str(base)]=dict(id=base,system=1,name='Fixed '+name+' state execution',states={str(remap(i)):dict(id=remap(i),name=name+' '+kind,kind=kind)for i,kind in g['states']},events={str(remap(i)):dict(id=remap(i),name=name+' '+label)for i,label in g['events']},transitions={str(remap(i)):dict(id=remap(i),name=name+' route '+str(i),source=remap(a),target=remap(b),event=None if ev is None else remap(ev),guard=None if guard=='' else 'Else' if guard=='else' else dict(Boolean=guard=='true'))for i,a,b,ev,guard in g['transitions']})
 for c in oracle['cases']:
  if c['graph']!=name:continue
  case=copy.deepcopy(c);case['machine']=base;case['events']=[remap(e)for e in c['events']];case['steps']=[{k:None if v is None else remap(v)for k,v in step.items()}for step in c['steps']];case['state']=remap(c['state']);status=case['status']
  if isinstance(status,dict):
   data=next(iter(status.values()));data['state']=remap(data['state'])
   if 'event'in data:data['event']=None if data['event']is None else remap(data['event']);data['transitions']=[remap(t)for t in data['transitions']]
  expected.append(case)
s['extensions']['namespaces']=json.loads(json.dumps(s['extensions']['namespaces'],sort_keys=True));s['next_id']=14000;s['model_revision']=123
p=d/'state-fixture.kernspan';p.write_bytes(wire(root));views['binding'].update(artifact_sha256=sha(p.read_bytes()),snapshot_sha256=sha(wire(s)));p.with_suffix('.mbse-views.json').write_bytes(wire(views));assert all(views.get(k,{})==original_views.get(k,{})for k in ['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams','state_machine_diagrams'])
(d/'EXPECTED.json').write_text(json.dumps(dict(preparation_only=True,machine_mapping=mapping,cases=expected,original_native_sha256=sha(source.read_bytes()),original_views_sha256=sha(side.read_bytes()),fixture_native_sha256=sha(p.read_bytes()),fixture_snapshot_sha256=sha(wire(s)),qualification='Only remapped P33a graph DTOs/nextID/modelrevision and bound identity added. All original native/type/activity/state7000/raw/opaque and seven presentation maps retained. Native Open must validate before execution acceptance.'),indent=2)+'\n');print(mapping)
