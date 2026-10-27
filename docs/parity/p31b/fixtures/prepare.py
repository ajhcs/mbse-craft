"""Materialize static execution fixtures. This is preparation, not native authoring or execution proof."""
from pathlib import Path
import json,hashlib,copy
r=Path(__file__).resolve().parents[4];d=Path(__file__).resolve().parent
source=r/'docs/parity/p30c/verification/native-04/run/activity-cold-copy.kernspan';vp=source.with_suffix('.mbse-views.json');original=json.loads(source.read_text());views=json.loads(vp.read_text());original_views=copy.deepcopy(views)
wire=lambda v:json.dumps(v,separators=(',',':'),ensure_ascii=False).encode();sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(wire(original['snapshot']))==views['binding']['snapshot_sha256']
root=copy.deepcopy(original);snapshot=root['snapshot'];activities=snapshot['extensions']['namespaces']['mbse-craft.types/1']['activities'];oracle=json.loads((r/'docs/parity/p31a/fixtures/control-traces.json').read_text());mapping={};oracles=[]
for index,(name,g)in enumerate(oracle['graphs'].items(),2):
 base=index*1000;remap=lambda old:old+base-1000
 nodes={str(remap(n)):dict(id=remap(n),kind=k,name=f'{name} {k}',partition=None)for n,k in g['nodes']};flows={str(remap(f)):dict(id=remap(f),name=f'{name} control {f}',kind='Control',source=remap(s),target=remap(t),guard=None if guard==''else 'Else'if guard=='else'else dict(Boolean=guard=='true'))for f,s,t,guard in g['flows']}
 activities[str(base)]=dict(id=base,system=1,name=f'Fixed {name} execution',partitions={},nodes=nodes,pins={},flows=flows);mapping[name]=base
 for case in oracle['cases']:
  if case['graph']!=name:continue
  status=copy.deepcopy(case['status'])
  if isinstance(status,dict):status['Deadlock']['node']=remap(status['Deadlock']['node'])
  oracles.append(dict(activity=base,graph=name,budget=case['budget'],cancel_after=case['cancel_after'],status=status,visits=[[remap(n),None if f is None else remap(f)]for n,f in case['visits']]))
# Namespace payloads are serde_json::Value sorted maps; retain Snapshot struct order.
snapshot['extensions']['namespaces']=json.loads(json.dumps(snapshot['extensions']['namespaces'],sort_keys=True));snapshot['next_id']=7000;snapshot['model_revision']=104
native=d/'control-fixture.kernspan';native.write_bytes(wire(root));views['binding'].update(artifact_sha256=sha(native.read_bytes()),snapshot_sha256=sha(wire(snapshot)));(d/'control-fixture.mbse-views.json').write_bytes(wire(views));assert all(views.get(k,{})==original_views.get(k,{})for k in['views','occurrences','edges','typed_ports','connector_edges','activity_diagrams'])
(d/'EXPECTED.json').write_text(json.dumps(dict(preparation_only=True,activity_mapping=mapping,expected_cases=oracles,original_native_sha256=sha(source.read_bytes()),original_views_sha256=sha(vp.read_bytes()),fixture_native_sha256=sha(native.read_bytes()),fixture_snapshot_sha256=sha(wire(snapshot)),qualification='Static graph DTO fixtures mapped from independent P31a tables; existing accepted activity1033/payloads/raw/native/type/opaque/presentation data retained. Native Open must validate before execution acceptance.'),indent=2)+'\n')
print(mapping)
