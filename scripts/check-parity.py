#!/usr/bin/env python3
"""Check parity planning/fixture integrity. Never execute product acceptance."""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / 'docs/parity'
FILES = ('roadmap', 'matrix', 'acceptance', 'baseline', 'sources', 'craft-apps', 'milestone1')


class InvalidPackage(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidPackage(message)


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f'duplicate JSON key: {key}')
        result[key] = value
    return result


def read_package():
    return {name: json.loads((PACKAGE / f'{name}.json').read_text(), object_pairs_hook=unique_pairs)
            for name in FILES}


def indexed(items, label):
    require(isinstance(items, list) and items, f'{label}: empty or invalid collection')
    result = {}
    for item in items:
        require(isinstance(item, dict) and isinstance(item.get('id'), str), f'{label}: missing ID')
        require(item['id'] not in result, f'{label}: duplicate ID {item["id"]}')
        result[item['id']] = item
    return result


def bound_file(path):
    require(isinstance(path, str), 'non-string local path')
    relative = Path(path)
    require(not relative.is_absolute() and '..' not in relative.parts, f'unsafe path: {path}')
    target = ROOT / relative
    require(target.is_file() and target.resolve().is_relative_to(ROOT.resolve()), f'missing/escaped input: {path}')
    return target


def validate(data, asset_override=None, check_sources=False, check_render=True):
    schemas = {'roadmap': 'roadmap', 'matrix': 'matrix', 'acceptance': 'acceptance',
               'baseline': 'baseline', 'sources': 'sources', 'craft-apps': 'craft-apps',
               'milestone1': 'milestone1'}
    for name, suffix in schemas.items():
        require(data[name].get('schema') == f'mbse-craft.parity-{suffix}/1', f'unsupported schema: {name}')
    roadmap = data['roadmap']
    prs = indexed(roadmap['prs'], 'PRs')
    milestones = indexed(roadmap['milestones'], 'milestones')
    require(list(prs) == [f'P{i:02}' for i in range(1, 59)], '58 agreed ordered PR positions required')
    require(list(milestones) == [f'M{i:02}' for i in range(1, 11)], 'ten milestones required')
    seen = set()
    for pid, pr in prs.items():
        require(pr['milestone'] in milestones and pr['acceptance'] and pr['owner_role'], f'{pid}: incomplete PR')
        require(all(dep in seen for dep in pr['depends_on']), f'{pid}: unknown/forward/cyclic dependency')
        seen.add(pid)
    require(all(m['exit_criteria'] for m in milestones.values()), 'milestone exit criteria missing')

    sources = indexed(data['sources']['sources'], 'sources')
    for source in sources.values():
        require(source['scope'], 'source scope missing')
        if 'commit' in source:
            require(re.fullmatch(r'[0-9a-f]{40}', source['commit']), 'invalid source commit')
            if check_sources:
                result = subprocess.run(['git', '-C', str(ROOT), 'cat-file', '-e', f'{source["commit"]}:{source["path"]}'],
                                        capture_output=True, check=False)
                require(result.returncode == 0, f'missing pinned local source: {source["id"]}')
                raw = subprocess.check_output(['git', '-C', str(ROOT), 'show', f'{source["commit"]}:{source["path"]}'])
                require(hashlib.sha256(raw).hexdigest() == source['sha256'], f'local source digest mismatch: {source["id"]}')
        elif 'blob_sha' in source:
            require(re.fullmatch(r'[0-9a-f]{40}', source['blob_sha']), 'invalid source blob')
        else:
            require(source.get('url', '').startswith('https://') and source.get('document_version'), 'unidentified vendor source')

    caps = indexed(data['matrix']['capabilities'], 'capabilities')
    require({x for x in caps if x.startswith('CAT-')} == {f'CAT-{i:03}' for i in range(1, 41)}, 'CAT-001 through CAT-040 required')
    require({x for x in caps if x.startswith('UI-')} == {f'UI-{i:03}' for i in range(1, 17)}, 'UI-001 through UI-016 required')
    require(len(caps) == 56, 'unexpected capability IDs')
    scenarios = indexed(data['acceptance']['scenarios'], 'acceptance scenarios')
    assets = indexed(data['acceptance']['assets'], 'assets')
    apps = data['craft-apps']['apps']
    app_names = [a['name'] for a in apps]
    require(len(app_names) == len(set(app_names)) == 13, 'initial thirteen named app references required')
    targets = {t['id'] for t in data['baseline']['comparison_targets']} | set(app_names)
    allowed_status = {'partial_bounded', 'not_implemented', 'unqualified'}
    for cid, cap in caps.items():
        require(cap['status'] in allowed_status and cap['acceptance_state'] == 'not_run', f'{cid}: unsupported acceptance claim')
        require(cap['status_scope'] and cap['completion_rule'] and cap['owner_role'], f'{cid}: missing scope/ownership')
        require(cap['planned_prs'] and all(p in prs for p in cap['planned_prs']), f'{cid}: missing PR')
        require(cap['source_refs'] and all(s in sources for s in cap['source_refs']), f'{cid}: missing source')
        require(all(t in targets for t in cap['target_refs']), f'{cid}: missing target')
        require(cap['acceptance_fixture_ids'] and all(f in scenarios for f in cap['acceptance_fixture_ids']), f'{cid}: missing fixture')
        for fid in cap['acceptance_fixture_ids']:
            require(cid in scenarios[fid]['capability_ids'], f'{cid}: fixture is not reciprocal')
    for fid, scenario in scenarios.items():
        require(scenario['execution_state'] == 'not_run' and scenario['kind'] == 'declarative_acceptance_specification', f'{fid}: forged product result')
        require(scenario['action'] and scenario['expected'] and scenario['failure_invariants'] and scenario['evidence_required'], f'{fid}: incomplete oracle')
        require(all(c in caps and fid in caps[c]['acceptance_fixture_ids'] for c in scenario['capability_ids']), f'{fid}: dangling/nonreciprocal capability')
        require(all(p in prs for p in scenario['requires_prs']), f'{fid}: dangling prerequisite')
        require(all(a in assets for a in scenario['input_asset_ids'] + scenario.get('native_seed_asset_ids', [])), f'{fid}: dangling asset')
    for app in apps:
        require(all(s in sources for s in app['source_refs']), 'app source missing')
        require(all(f in scenarios for f in app['comparison_fixture_ids']), 'app fixture missing')
        require(app['reference_build_status'] == 'not_run_not_pinned_for_interactive_comparison', 'invented reference qualification')

    for aid, asset in assets.items():
        require(re.fullmatch(r'[0-9a-f]{64}', asset['sha256']), f'{aid}: missing SHA256')
        raw = (asset_override or {}).get(aid)
        if raw is None:
            raw = bound_file(asset['path']).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == asset['sha256'], f'{aid}: changed input bytes')
    seed = json.loads(bound_file(assets['ASSET-NATIVE']['path']).read_text())
    require(seed['schema'] == 'kernspan.stage1.qa-fixture/1', 'native seed format relabeled')
    require(any(d['recordId'] == 301 and d['rawText'] == '9e' and d['acceptedNumeric'] == 95 for d in seed['rawDrafts']), 'seed draft oracle changed')
    engineering = json.loads(bound_file(assets['ASSET-ENGINEERING']['path']).read_text())
    numeric = {o['id']: o for o in engineering['numeric_oracles']}
    require(numeric['speed']['inputs']['distance_m'] / numeric['speed']['inputs']['duration_s'] == numeric['speed']['expected']['value'] == 15, 'speed oracle incorrect')
    require(numeric['length']['inputs']['length_m'] / 1000 == numeric['length']['expected']['value'] == 2.5, 'length oracle incorrect')
    calc = numeric['assessment']
    percent = calc['inputs']['numerator'] / calc['inputs']['denominator'] * 100
    require(percent == calc['expected']['value'] == 87.5 and calc['expected']['target_met'] == (percent >= calc['inputs']['target_percent']), 'assessment oracle incorrect')
    require(numeric['zero']['inputs']['denominator'] == 0 and numeric['zero']['expected']['outcome'] == 'invalid', 'zero-denominator oracle incorrect')
    model = engineering['planned_model']
    count_keys = {'planned_types':'types', 'planned_instances':'instances', 'planned_ports':'ports', 'planned_requirements':'requirements', 'planned_views':'views'}
    for key, collection in count_keys.items():
        require(engineering['known_counts'][key] == len(model[collection]), f'incorrect count: {key}')
    reqs = {r['id'] for r in model['requirements']}
    traces = model['trace_fixture']
    require(engineering['known_counts']['planned_exchanges'] == 1 and model['exchange']['id'] == 'EX-SAMPLE', 'exchange count oracle incorrect')
    require(engineering['known_counts']['declared_satisfy_links'] == len(traces['satisfy']) and engineering['known_counts']['declared_verify_links'] == len(traces['verify']), 'trace count oracle incorrect')
    require(set(traces['expected_unsatisfied']) == reqs - {x['requirement'] for x in traces['satisfy']}, 'coverage oracle incorrect')
    require(set(traces['expected_unverified']) == reqs - {x['requirement'] for x in traces['verify']}, 'verification-gap oracle incorrect')

    gates = indexed(data['matrix']['unresolved_scope'], 'scope gates')
    require(set(gates) == {'GATE-EDITION','GATE-STANDARDS','GATE-PORTFOLIO','GATE-CRAFT-EXPANSION','GATE-PLATFORMS','GATE-RESOURCING'}, 'scope blocker removed or changed')
    require(all(g['planned_prs'] and all(p in prs for p in g['planned_prs']) for g in gates.values()), 'scope gate has unknown owner PR')
    m1 = indexed(data['milestone1']['vectors'], 'Milestone 1 vectors')
    require(set(m1) == {'M1-AUTHOR','M1-PAIR','M1-RECOVERY','M1-NAV','M1-OCCURRENCE','M1-SOURCE'}, 'Milestone 1 integrated vectors incomplete')
    for vec in m1.values():
        require(vec['execution_state'] == 'not_run' and vec['expected'] and vec['steps'], 'Milestone 1 vector misrepresents execution')
        require(all(p in prs for p in vec['requires_prs']) and all(f in scenarios for f in vec['fixture_ids']), 'Milestone 1 dangling reference')
    # Facts about original XML fixture bytes only. This is not a product parser test.
    xmi_id = '{http://www.omg.org/XMI}id'
    primary = ET.fromstring(bound_file(assets['ASSET-SOURCE-PRIMARY']['path']).read_bytes())
    other = ET.fromstring(bound_file(assets['ASSET-SOURCE-OTHER']['path']).read_bytes())
    require(any(n.get(xmi_id) == 'SYN-DUP' and n.get('name') == 'Primary sensor' for n in primary), 'primary local-ID oracle incorrect')
    require(any(n.get(xmi_id) == 'SYN-DUP' and n.get('name') == 'Companion sensor' for n in other), 'companion local-ID oracle incorrect')
    require([n.get('href') for n in primary.iter() if n.get('href')] == ['source-other.xmi#SYN-DUP','missing.xmi#SYN-MISSING'], 'source references changed')
    require(any(n.get('{urn:mbse-craft:synthetic:unknown:1}tag') == 'preserve-me' for n in primary), 'unknown-metadata fixture missing')
    require(data['baseline']['base_commit'] == '5b907c333d423df60de7d539da2a0900941825aa', 'implementation baseline changed')
    if check_render:
        rendered = (ROOT / 'docs/PARITY_ROADMAP.md').read_text()
        for pid, pr in prs.items():
            require(f"{int(pid[1:])}. **{pr['title']}.** Acceptance: {pr['acceptance'][0]}" in rendered, f'roadmap drift: {pid}')
        for i, m in enumerate(milestones.values(), 1):
            require(f"## Milestone {i}: {m['title']}" in rendered and f"**Milestone exit:** {m['exit_criteria'][0]}" in rendered, f'milestone rendering drift: {i}')
        matrix_text = (PACKAGE / 'MATRIX.md').read_text()
        for cid, cap in caps.items():
            require(matrix_row(cap) in matrix_text, f'matrix rendering drift: {cid}')
    return len(prs), len(caps), len(scenarios)


def matrix_row(cap):
    fixtures = ', '.join(cap['acceptance_fixture_ids'])
    prs = ', '.join(cap['planned_prs'])
    return f"| {cap['id']} | {cap['title']} | {cap['status']} | {prs} | {fixtures} |"


def self_test(data):
    cases = []
    def add(name, mutate):
        altered = copy.deepcopy(data)
        mutate(altered)
        cases.append((name, altered, None))
    add('duplicate capability ID', lambda d: d['matrix']['capabilities'].append(copy.deepcopy(d['matrix']['capabilities'][0])))
    add('forward dependency', lambda d: d['roadmap']['prs'][1]['depends_on'].append('P58'))
    add('missing scenario', lambda d: d['matrix']['capabilities'][0]['acceptance_fixture_ids'].append('AC-MISSING'))
    add('forged product pass', lambda d: d['acceptance']['scenarios'][0].update(execution_state='passed'))
    add('forged capability accepted', lambda d: d['matrix']['capabilities'][0].update(acceptance_state='accepted'))
    add('removed scope blocker', lambda d: d['matrix']['unresolved_scope'].pop())
    add('unknown source', lambda d: d['matrix']['capabilities'][0]['source_refs'].append('SRC-MISSING'))
    add('unsafe fixture path', lambda d: d['acceptance']['assets'][0].update(path='../../outside.json'))
    add('unknown schema', lambda d: d['matrix'].update(schema='mbse-craft.parity-matrix/99'))
    cases.append(('tampered source bytes', copy.deepcopy(data), {'ASSET-SOURCE-PRIMARY': b'<changed/>'}))
    for name, altered, override in cases:
        try:
            validate(altered, override, check_sources=False, check_render=False)
        except InvalidPackage:
            continue
        raise InvalidPackage(f'negative case was accepted: {name}')
    try:
        json.loads('{"id":1,"id":2}', object_pairs_hook=unique_pairs)
    except InvalidPackage:
        pass
    else:
        raise InvalidPackage('duplicate JSON keys accepted')
    return len(cases) + 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--self-test', action='store_true', help='also reject malformed/forged package variants in memory')
    parser.add_argument('--verify-local-sources', action='store_true', help='also require cited local-only historical Git objects; these are not present in a normal fresh clone')
    args = parser.parse_args()
    try:
        data = read_package()
        prs, caps, scenarios = validate(data, check_sources=args.verify_local_sources)
        negative = self_test(data) if args.self_test else 0
    except (InvalidPackage, KeyError, TypeError, OSError, json.JSONDecodeError, ET.ParseError) as exc:
        print(f'PARITY PACKAGE INVALID: {exc}', file=sys.stderr)
        return 1
    print(f'PACKAGE CONSISTENCY PASS: {prs} PR positions; {caps} capabilities; {scenarios} NOT-RUN acceptance specifications; {negative} negative cases rejected.')
    print('No application tests, native workflows, interoperability, performance or engineering acceptance were executed.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
