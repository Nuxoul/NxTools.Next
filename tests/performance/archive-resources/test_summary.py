"""Summary contract regressions using immutable synthetic attempt files."""
import hashlib
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('resource_summary', pathlib.Path(__file__).with_name('summarize.py'))
summary = importlib.util.module_from_spec(spec); spec.loader.exec_module(summary)


class SummaryTests(unittest.TestCase):
    def build(self, count=127):
        temporary = tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR']); self.addCleanup(temporary.cleanup)
        scratch = pathlib.Path(temporary.name); root = scratch / 'attempts'; root.mkdir()
        names = [f'fixture-{index}' for index in range(count)]
        for name in ('environment.json', 'fixture-summary.json'):
            (root / name).write_text('{}')
        (scratch / 'resource-stable-csharp-selection.json').write_text(json.dumps(names))
        (scratch / 'resource-stable-csharp-list.log').write_text(''.join(f'LIST Fixture | {name} | test\n' for name in names))
        (scratch / 'resource-stable-csharp-run.log').write_text(''.join(f'RUN {name}\nPASS {name}\n' for name in names) +
            f'SUMMARY selected={count} run={count} pass={count} skip=0 fail=0 not_run=0\n')
        for label in ('archive', 'protocol', 'ipc', 'extensions', 'clipboard'):
            (scratch / f'resource-stable-{label}.log').write_text('test result: ok. 2 passed; 0 failed; 0 ignored;\n')
        (scratch / 'resource-verification-ledger.txt').write_text('synthetic ledger')
        return root, scratch

    def current_attempt(self, stdout):
        return dict(schema=2, scenario='runner100k', exit_code=1, stdout=stdout,
                    operation={'status': 'Stopped', 'reason': 'driver_exception'},
                    measurement={'status': 'Incomplete', 'reasons': ['actual_worker:Unavailable']},
                    stop={'requested': True, 'forced': False, 'reason': 'driver_exception'},
                    cleanup={'status': 'Confirmed', 'confirmed': True}, resource_verdict='Stopped',
                    failure='driver_exception:JSONDecodeError')

    def collect_preserving_inputs(self, root, scratch):
        before = {path: path.read_bytes() for path in scratch.rglob('*') if path.is_file()}
        hashes = {path: hashlib.sha256(data).hexdigest() for path, data in before.items()}
        try:
            return summary.collect(root, scratch)
        finally:
            after = {path: path.read_bytes() for path in scratch.rglob('*') if path.is_file()}
            self.assertEqual(before, after)
            self.assertEqual(hashes, {path: hashlib.sha256(data).hexdigest() for path, data in after.items()})

    def test_malformed_resource_retains_valid_events_and_later_attempt(self):
        root, scratch = self.build(2)
        broken = self.current_attempt('probe heading\nRESOURCE {"phase":"ready"}\n'
                                      'RESOURCE not-json\nRESOURCE {"phase":"terminal"}\n')
        healthy = dict(schema=2, scenario='engine100k', exit_code=0, failure=None,
                       stdout='RESOURCE {"entries":5}\n', operation={'status': 'Succeeded'},
                       measurement={'status': 'Complete'}, stop={'requested': False, 'forced': False},
                       cleanup={'status': 'Confirmed', 'confirmed': True}, resource_verdict='PASS')
        for name, original in (('01-broken.json', broken), ('02-healthy.json', healthy)):
            (root / name).write_text(json.dumps(original), encoding='utf-8')
        result = self.collect_preserving_inputs(root, scratch)
        self.assertEqual(['01-broken.json', '02-healthy.json'], [item['record_file'] for item in result['attempts']])
        first, later = result['attempts']
        self.assertEqual([{'phase': 'ready'}, {'phase': 'terminal'}], first['events'])
        self.assertEqual({'status': 'Incomplete', 'error_count': 1,
                          'errors': [{'line': 3, 'category': 'invalid_resource_json'}],
                          'errors_truncated': False}, first['summary_parse'])
        self.assertEqual([{'entries': 5}], later['events'])
        self.assertEqual({'status': 'Complete', 'error_count': 0, 'errors': [],
                          'errors_truncated': False}, later['summary_parse'])
        for original, record in ((broken, first), (healthy, later)):
            for key, value in original.items():
                self.assertEqual(value, record[key], key)
            self.assertEqual(hashlib.sha256((root / record['record_file']).read_bytes()).hexdigest(), record['sha256'])
        self.assertEqual(('Complete', 2, 2), tuple(result['csharp'][key] for key in ('status', 'selected', 'passed')))

    def test_retained_driver_parse_failure_does_not_drop_later_attempt(self):
        root, scratch = self.build(2)
        broken = self.current_attempt('RESOURCE not-json\n')
        later = self.current_attempt('RESOURCE {"phase":"ready"}\n')
        for name, original in (('01-broken.json', broken), ('02-later.json', later)):
            (root / name).write_text(json.dumps(original), encoding='utf-8')
        records = self.collect_preserving_inputs(root, scratch)['attempts']
        self.assertEqual(2, len(records))
        self.assertEqual([], records[0]['events'])
        self.assertEqual({'status': 'Incomplete', 'error_count': 1,
                          'errors': [{'line': 1, 'category': 'invalid_resource_json'}],
                          'errors_truncated': False}, records[0]['summary_parse'])
        self.assertEqual([{'phase': 'ready'}], records[1]['events'])
        self.assertEqual('Complete', records[1]['summary_parse']['status'])
        for original, record in zip((broken, later), records):
            for key, value in original.items():
                self.assertEqual(value, record[key], key)

    def test_truncated_resource_marks_summary_incomplete_without_rewriting_original_pass(self):
        root, scratch = self.build(2)
        original = self.current_attempt('RESOURCE {"phase":"ready"}\nRESOURCE {"phase":')
        original.update(exit_code=0, failure=None, operation={'status': 'Succeeded'},
                        measurement={'status': 'Complete'}, stop={'requested': False, 'forced': False},
                        resource_verdict='PASS')
        (root / 'truncated.json').write_text(json.dumps(original), encoding='utf-8')
        record = self.collect_preserving_inputs(root, scratch)['attempts'][0]
        self.assertEqual([{'phase': 'ready'}], record['events'])
        self.assertEqual({'status': 'Incomplete', 'error_count': 1,
                          'errors': [{'line': 2, 'category': 'invalid_resource_json'}],
                          'errors_truncated': False}, record['summary_parse'])
        for key, value in original.items():
            self.assertEqual(value, record[key], key)

    def test_resource_parse_diagnostics_have_fixed_bound_and_no_payload_copy(self):
        for count in (16, 17, 4096):
            with self.subTest(bad_lines=count):
                root, scratch = self.build(2)
                payload = 'payload-must-not-be-copied-' + 'x' * 131072
                original = self.current_attempt('RESOURCE ' + payload + '\n' +
                    'RESOURCE not-json\n' * (count - 1) + 'RESOURCE {"phase":"after-errors"}\n')
                (root / 'many-bad-lines.json').write_text(json.dumps(original), encoding='utf-8')
                record = self.collect_preserving_inputs(root, scratch)['attempts'][0]
                self.assertEqual([{'phase': 'after-errors'}], record['events'])
                diagnostics = record['summary_parse']
                self.assertEqual('Incomplete', diagnostics['status'])
                self.assertEqual(count, diagnostics['error_count'])
                self.assertEqual([{'line': line, 'category': 'invalid_resource_json'}
                                  for line in range(1, 17)], diagnostics['errors'])
                self.assertEqual(count > 16, diagnostics['errors_truncated'])
                self.assertLess(len(json.dumps(diagnostics)), 2048)
                self.assertNotIn('payload-must-not-be-copied', json.dumps(diagnostics))
                for key, value in original.items():
                    self.assertEqual(value, record[key], key)

    def test_legacy_exit_zero_is_not_new_measurement_pass_and_original_time_is_preserved(self):
        root, scratch = self.build()
        original = {'scenario': 'runner100k', 'exit_code': 0, 'failure': None, 'processes': [],
                    'stdout': 'RESOURCE {"first_batch_ms":123,"full_terminal_ms":456}\n'}
        path = root / 'legacy.json'; path.write_text(json.dumps(original)); before = path.read_bytes()
        record = summary.collect(root, scratch)['attempts'][0]
        self.assertEqual('Legacy', record.get('measurement', {}).get('status'))
        self.assertNotEqual('PASS', record.get('resource_verdict'))
        self.assertEqual('setup-inclusive', record.get('timing_origin'))
        self.assertEqual(123, record['events'][0]['first_batch_ms'])
        self.assertEqual(before, path.read_bytes())

    def test_current_selection_count_not_historical_constant(self):
        root, scratch = self.build(2)
        result = summary.collect(root, scratch)
        self.assertEqual(2, result['csharp']['selected'])
        self.assertEqual(2, result['csharp']['passed'])

    def test_incomplete_forced_and_cleanup_reasons_survive_summary(self):
        root, scratch = self.build()
        original = dict(schema=2, scenario='runner100k', exit_code=0, operation={'status': 'Succeeded'},
                        measurement={'status': 'Incomplete', 'reasons': ['actual_worker:Unavailable']},
                        stop={'forced': True, 'reason': 'deadline'}, cleanup={'confirmed': False, 'status': 'Unknown'},
                        resource_verdict='Failed', failure='deadline')
        (root / 'forced.json').write_text(json.dumps(original))
        record = summary.collect(root, scratch)['attempts'][0]
        for key in ('operation', 'measurement', 'stop', 'cleanup', 'resource_verdict', 'failure'):
            self.assertEqual(original[key], record[key])


class InterruptedSummaryTests(unittest.TestCase):
    build = SummaryTests.build
    def test_interrupted_regression_log_is_recorded_not_erased(self):
        root, scratch = self.build(2)
        (root / 'attempt.json').write_text(json.dumps(dict(schema=2, scenario='engine100k', operation={'status':'Succeeded'},
            measurement={'status':'Incomplete'}, stop={'forced':False}, cleanup={'confirmed':False}, resource_verdict='Incomplete')))
        (scratch / 'resource-stable-csharp-run.log').write_text('RUN fixture-0\nPASS fixture-0\nRUN fixture-1\n')
        result = summary.collect(root, scratch)
        self.assertEqual(1, len(result['attempts']))
        self.assertEqual('Incomplete', result['csharp']['status'])
        self.assertEqual((2, 2, 1, 0), tuple(result['csharp'][key] for key in ('selected','run','passed','not_run')))
        self.assertFalse(result['csharp']['summary_present'])


if __name__ == '__main__':
    unittest.main()
