"""Collect immutable attempts, exact C# identities and Rust counts; never run a resource scenario."""
import argparse
import hashlib
import json
import pathlib
import re

RESOURCE_PARSE_ERROR_LIMIT = 16


def read_log(path):
    data = path.read_bytes()
    return data.decode('utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig', errors='replace').replace('\r\n', '\n')

def collect(root, scratch):
    records = []
    for path in sorted(root.glob('*.json')):
        item = json.loads(path.read_text(encoding='utf-8-sig'))
        if 'scenario' not in item:
            continue
        item['record_file'] = path.name
        item['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        item['events'] = []
        parse_errors = []
        parse_error_count = 0
        for line_number, line in enumerate(item.get('stdout', '').splitlines(), 1):
            if not line.startswith('RESOURCE '):
                continue
            try:
                event = json.loads(line.removeprefix('RESOURCE '))
            except json.JSONDecodeError:
                parse_error_count += 1
                if len(parse_errors) < RESOURCE_PARSE_ERROR_LIMIT:
                    parse_errors.append(dict(line=line_number, category='invalid_resource_json'))
            else:
                item['events'].append(event)
        # Summary completeness is independent of the attempt's original verdict/facts.
        item['summary_parse'] = dict(status='Incomplete' if parse_error_count else 'Complete',
                                     error_count=parse_error_count, errors=parse_errors,
                                     errors_truncated=parse_error_count > len(parse_errors))
        required = ('operation', 'measurement', 'stop', 'cleanup', 'resource_verdict')
        if item.get('schema') != 2 or any(key not in item for key in required):
            item['legacy_original'] = {key: item[key] for key in required if key in item}
            item['operation'] = dict(status='Legacy', exit_code=item.get('exit_code'))
            item['measurement'] = dict(status='Legacy', reasons=['missing_resource_tool_contract_fields'])
            item['cleanup'] = dict(status='Unknown', confirmed=False, reason='legacy_no_owner_confirmation')
            item['stop'] = dict(status='Legacy', forced='Unknown')
            item['resource_verdict'] = 'Legacy'
            item['timing_origin'] = 'setup-inclusive' if item['scenario'] in ('runner100k', 'cancel1gib', 'cancel1gib-hold') else 'legacy_scenario_specific'
        records.append(item)
    expected = json.loads((scratch / 'resource-stable-csharp-selection.json').read_text(encoding='utf-8-sig'))
    listed_text = read_log(scratch / 'resource-stable-csharp-list.log')
    run_text = read_log(scratch / 'resource-stable-csharp-run.log')
    listed = re.findall(r'^LIST Fixture \| (.*?) \| ', listed_text, re.M)
    ran = re.findall(r'^RUN (.*)$', run_text, re.M)
    passed = re.findall(r'^PASS (.*)$', run_text, re.M)
    expected_count = len(expected)
    assert expected_count > 0 and len(set(expected)) == expected_count
    skipped = re.findall(r'^SKIP (.*?): ', run_text, re.M)
    failed = re.findall(r'^FAIL (.*?): ', run_text, re.M)
    match = re.search(r'^SUMMARY selected=(\d+) run=(\d+) pass=(\d+) skip=(\d+) fail=(\d+) not_run=(\d+)', run_text, re.M)
    observed = [expected_count, len(ran), len(passed), len(skipped), len(failed), max(0, expected_count - len(ran))]
    exact = set(expected) == set(listed) and listed == ran == passed and len(ran) == expected_count
    summary_matches = bool(match and list(map(int, match.groups())) == observed)
    csharp_status = 'Complete' if exact and summary_matches and not skipped and not failed else 'Failed' if failed else 'Incomplete'
    csharp = dict(status=csharp_status, selected=expected_count, run=len(ran), passed=len(passed), skip=len(skipped),
                  fail=len(failed), not_run=observed[-1], summary_present=bool(match), summary_matches=summary_matches,
                  exact_names=ran, listed_names=listed, passed_names=passed, skipped_names=skipped, failed_names=failed,
                  large_zip64_run='archive ZIP creation fixture large ZIP64 streaming' in ran)
    rust = {}
    for label in ('archive', 'protocol', 'ipc', 'extensions', 'clipboard'):
        text = read_log(scratch / f'resource-stable-{label}.log')
        groups = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        totals = [sum(int(row[index]) for row in groups) for index in range(3)]
        rust[label] = dict(status='Complete' if totals[0] > 0 and totals[1:] == [0, 0] else 'Failed' if totals[1] else 'Incomplete',
                           passed=totals[0], failed=totals[1], ignored=totals[2], groups=len(groups), nonempty_groups=sum(int(row[0]) > 0 for row in groups))
    return dict(environment=json.loads((root / 'environment.json').read_text(encoding='utf-8-sig')),
                fixtures=json.loads((root / 'fixture-summary.json').read_text()), attempts=records,
                csharp=csharp, rust=rust,
                ledger=read_log(scratch / 'resource-verification-ledger.txt'),
                caveats=['Debug only; no strict cold/warm-cache guarantee', '20ms samples may miss peaks; actual intervals in records',
                         'runner100k external sampler failure retained; internal exact-handle samples in events',
                         '1GiB product first-batch gate failed before cancellation; not passed',
                         'worker cancellation receive/observe times not exposed; exit/cleanup are distinct',
                         'tool-guard-failure exit101 is intentional test-tool cleanup evidence, not product PASS'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=pathlib.Path)
    parser.add_argument('output', type=pathlib.Path)
    args = parser.parse_args()
    result = collect(args.root, args.root.parent)
    with args.output.open('x', encoding='utf-8') as target:
        json.dump(result, target, ensure_ascii=False, indent=2)
    print(json.dumps(dict(csharp=result['csharp'] | {'exact_names': 'reconciled'}, rust=result['rust'], attempts=len(result['attempts']))))
