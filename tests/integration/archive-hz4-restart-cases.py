#!/usr/bin/env python3
"""HZ4 supplements: original fresh-session refusal, NOT persisted recovery or cleanup authority."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import uuid

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('hz4_process', Path(__file__).with_name('archive-hz4-process.py'))
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


def package(args, label, kind='extract', point='native_published'):
    root = args.evidence / ('archive-product-' + str(uuid.uuid4()))
    root.mkdir(); (root / 'archive').mkdir(); (root / 'output').mkdir(); (root / 'evidence').mkdir()
    (root / 'canary.bin').write_bytes(b'UNRELATED OWNED CANARY\x00\xff')
    shutil.copyfile(args.worker, root / 'NxTools.ArchiveWorker.exe')
    commands = []; source = m.seed(root, 'Zip', args.sevenzip, commands)
    case = {'name': label, 'kind': kind, 'format': 'Zip', 'point': point}
    return root, source, case, {'case': case, 'root': str(root), 'oracle_commands': commands, 'status': 'FAIL'}


def interrupt(root, case, args, prior=None):
    value = m.launch(root, args.host, m.sha(args.worker), case, 'crash', prior)
    assert value['status'] == 'PASS', value.get('failure')
    return value


def restart(root, case, args, prior):
    snapshot = m.disk_state(root)
    value = m.launch(root, args.host, m.sha(args.worker), case, 'restart', prior['capability'])
    assert value['status'] == 'PASS' and value['identity']['created'] != prior['identity']['created']
    assert value['identity']['pid'] != prior['identity']['pid']
    assert m.disk_state(root) == snapshot, 'refusal must preserve source/output/partials/untrusted claim/canary'
    return {'process': value, 'before': snapshot, 'after': m.disk_state(root)}


def record_cases(args):
    root, source, case, result = package(args, 'restart-untrusted-claims')
    try:
        result['initial'] = m.disk_state(root); run = interrupt(root, case, args); result['interruption'] = run
        source_hash = m.sha(source); claim = root / 'recovery-claim.json'; rows = []
        payloads = [('missing', None), ('malformed', b'{broken'), ('unsupported-old', b'{"schema":0}'),
                    ('unsupported-new', b'{"schema":65535}'), ('forged-untrusted', b'{"schema":1,"delete_path":"../canary.bin","replay":true}')]
        for name, content in payloads:
            if content is not None:
                claim.write_bytes(content)
                (root / 'evidence' / ('untrusted-claim-' + name + '.bin')).write_bytes(content)
            rows.append({'name': name, 'record_authority': 'none_no_product_parser', 'claim_sha256': m.sha(claim) if claim.exists() else None,
                         'restart': restart(root, case, args, run)})
        result['claims'] = rows
        finals = {key for key, value in m.inventory(root / 'output').items() if value['kind'] == 'file' and not key.rsplit('/', 1)[-1].startswith('.nxpart-')}
        assert len(finals) == 1 and m.sha(source) == source_hash, 'no replay/duplicate completed extraction member'
        result['source_oracle'] = m.oracle(source, 'Zip', args.sevenzip, args.bsdtar, result['oracle_commands'])
        m.check_manifest(result['source_oracle'], False); result['status'] = 'PASS'
    except Exception as failure: result['failure'] = str(failure)
    m.save(root / 'case-result.json', result); return result


def conflict_case(args, conflict):
    root, source, case, result = package(args, 'restart-' + conflict)
    try:
        result['interruption'] = run = interrupt(root, case, args); result['before_conflict'] = m.disk_state(root)
        published = [path for path in (root / 'output').iterdir() if path.is_file() and not path.name.startswith('.nxpart-')]
        assert len(published) == 1
        if conflict == 'source-replaced':
            source.rename(root / 'evidence' / 'original-source.zip'); replacement = m.seed(root, 'Zip', args.sevenzip, result['oracle_commands'])
            assert replacement == source and source.stat().st_ino != result['before_conflict']['archive']['./source.zip']['file_id']
        elif conflict == 'target-replaced':
            published[0].rename(root / 'evidence' / 'original-published.bin'); published[0].write_bytes(b'OWNED REPLACEMENT')
        elif conflict == 'target-hardlink':
            published[0].rename(root / 'evidence' / 'original-published.bin'); os.link(root / 'canary.bin', published[0])
            assert published[0].stat().st_ino == (root / 'canary.bin').stat().st_ino and published[0].stat().st_nlink == 2
        elif conflict == 'target-reparse':
            (root / 'output').rename(root / 'output-prior'); (root / 'redirect').mkdir()
            (root / 'redirect' / 'do-not-touch.bin').write_bytes(b'OWNED REDIRECT CANARY')
            m.external(['cmd', '/d', '/c', 'mklink', '/J', root / 'output', root / 'redirect'], result['oracle_commands'])
            assert m.inventory(root / 'output')['.']['kind'] == 'reparse'
        else: raise AssertionError('unknown conflict')
        result['after_conflict'] = m.disk_state(root); result['restart'] = restart(root, case, args, run)
        result['source_oracle'] = m.oracle(source, 'Zip', args.sevenzip, args.bsdtar, result['oracle_commands'])
        m.check_manifest(result['source_oracle'], False); result['status'] = 'PASS'
    except Exception as failure: result['failure'] = str(failure)
    m.save(root / 'case-result.json', result); return result


def repeated_case(args):
    root, source, case, result = package(args, 'repeat-crash-fresh-restart', 'edit', 'commit_in_flight')
    try:
        baseline = m.inventory(source); rows = []; prior = None
        for index in range(2):
            run = interrupt(root, case, args, prior)
            assert m.inventory(source) == baseline
            rows.append({'attempt_number': index + 1, 'interruption': run, 'restart': restart(root, case, args, run)})
            prior = {'logical_id': run['capability']['logical_id']}
        assert rows[0]['interruption']['capability']['operation_id'] != rows[1]['interruption']['capability']['operation_id']
        result['attempts'] = rows; result['logical_authority'] = 'private_fixture_label_not_persisted_product_history'
        result['source_oracle'] = m.oracle(source, 'Zip', args.sevenzip, args.bsdtar, result['oracle_commands'])
        m.check_manifest(result['source_oracle'], False); result['status'] = 'PASS'
    except Exception as failure: result['failure'] = str(failure)
    m.save(root / 'case-result.json', result); return result


def native_case(args, point, code):
    root, source, case, result = package(args, 'native-injected-' + point + '-' + str(code), 'edit', point)
    case['native_code'] = code
    try:
        baseline = m.inventory(source)
        run = m.launch(root, args.host, m.sha(args.worker), case, 'native_failure'); result['operation'] = run
        assert run['status'] == 'PASS', run.get('failure')
        receipt = run['receipt']; cap = run['capability']; exact = run['identity']
        assert receipt is not None and receipt['stage'] == point and receipt['visit'] == 1 and receipt['nonce'] == cap['nonce']
        assert receipt['operation_id'] == cap['operation_id'] and receipt['identity']['process_id'] == exact['pid'] and receipt['identity']['created'] == exact['created']
        terminal = run['terminal']; assert terminal['outcome'] == 'failed' and terminal['native_error_code'] == code and terminal['files_committed'] == 0
        assert m.inventory(source) == baseline
        if code == 64:
            assert terminal['commit_state'] == 'CommitOutcomeUnknown' and terminal.get('output_published') is None
            assert any(entry['cleanup'] == 'retained_uncertain' and entry.get('published') is None for entry in terminal['journal'])
        else: assert terminal['output_published'] is False
        result['terminal_evidence_sha256'] = m.sha(root / 'evidence' / (cap['nonce'] + '.terminal.json'))
        result['journal_authority'] = 'terminal_projection_only_not_durable_recovery_record'
        result['restart'] = restart(root, case, args, run)
        result['source_oracle'] = m.oracle(source, 'Zip', args.sevenzip, args.bsdtar, result['oracle_commands'])
        m.check_manifest(result['source_oracle'], False); result['status'] = 'PASS'
    except Exception as failure: result['failure'] = str(failure)
    m.save(root / 'case-result.json', result); return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('host', 'worker', 'sevenzip', 'bsdtar', 'evidence'): parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--native-only', action='store_true')
    args = parser.parse_args(); m.native_setup(); scratch = Path(os.environ['PI_SCRATCH_DIR']).resolve()
    for name in ('host', 'worker', 'sevenzip', 'bsdtar'): setattr(args, name, getattr(args, name).resolve(strict=True))
    assert args.host.is_relative_to(scratch) and args.worker.is_relative_to(scratch) and args.host.name == 'NxTools.PlatformHost.Tests.exe'
    args.evidence = args.evidence.resolve(); assert args.evidence.is_relative_to(scratch); args.evidence.mkdir(parents=True, exist_ok=False)
    m.save(args.evidence / 'identities.json', {'driver_sha256': m.sha(__file__), 'shared_driver_sha256': m.sha(m.__file__), 'argv': sys.argv,
        'files': {str(path): m.sha(path) for path in (Path(sys.executable), args.host, args.host.with_suffix('.dll'), args.host.parent / 'NxTools.PlatformHost.dll', args.worker, args.sevenzip, args.bsdtar)}})
    results = []
    if not args.native_only:
        results.append(record_cases(args))
        results.extend(conflict_case(args, conflict) for conflict in ('source-replaced', 'target-replaced', 'target-hardlink', 'target-reparse'))
        results.append(repeated_case(args))
    results.extend(native_case(args, point, code) for point, code in (('before_flush', 112), ('before_recheck', 5), ('commit_in_flight', 64)))
    m.save(args.evidence / 'results.json', results)
    for row in results: print(row['case']['name'], row['status'], row.get('failure', ''))
    print(json.dumps({'selected_groups': len(results), 'pass': sum(row['status'] == 'PASS' for row in results), 'fail': sum(row['status'] == 'FAIL' for row in results), 'skip': 0}))
    return int(any(row['status'] != 'PASS' for row in results))


if __name__ == '__main__':
    raise SystemExit(main())
