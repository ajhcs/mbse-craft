"""Controller ownership regressions using mocked drivers; no GUI is launched."""
import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('m1_session', Path(__file__).with_name('m1_native_session.py'))
session = importlib.util.module_from_spec(spec)
spec.loader.exec_module(session)


class SessionOwnership(unittest.TestCase):
    def setUp(self):
        self.previous = session.NativeM1.instances
        session.NativeM1.instances = []
        self.receipt = dict(status='active', runs=[])

    def tearDown(self):
        session.NativeM1.instances = self.previous

    def driver(self, name, running=False, normal=True, evidence=None):
        evidence = self.receipt['runs'] if evidence is None else evidence
        record = dict(run=name)
        if normal:
            record.update(normal_app_exit=0, driver_exit=0, owned_processes_absent=True)
        child = Mock()
        child.poll.return_value = None if running else 0
        child.wait.return_value = 0
        driver = SimpleNamespace(run=Path(name), child=child, record=record, evidence=evidence)
        evidence.append(record)
        session.NativeM1.instances.append(driver)
        return driver

    def test_second_live_driver_blocks_finish_after_first_exits(self):
        self.driver('first')
        self.driver('second', running=True, normal=False)
        self.assertEqual(session.running_runs(self.receipt), ['second'])
        with self.assertRaisesRegex(AssertionError, 'every app'):
            session.finish_runs(self.receipt)
        self.assertEqual(self.receipt['status'], 'active')

    def test_driver_exit_without_verified_normal_app_close_cannot_finish(self):
        self.driver('terminated-driver', normal=False)
        with self.assertRaisesRegex(AssertionError, 'Normal cleanup'):
            session.finish_runs(self.receipt)

    def test_every_run_must_be_owned_and_closed_before_success(self):
        self.driver('first')
        self.driver('second')
        session.finish_runs(self.receipt)
        self.assertEqual(self.receipt['status'], 'actions_completed_review_pending')
        self.receipt['status'] = 'active'
        self.receipt['runs'].append(dict(run='unowned'))
        with self.assertRaisesRegex(AssertionError, 'owned driver'):
            session.finish_runs(self.receipt)

    def test_interrupted_cleanup_visits_all_owned_runs_despite_an_error(self):
        first = self.driver('first', running=True, normal=False)
        first.child.stdin.write.side_effect = BrokenPipeError('injected')
        second = self.driver('second', running=True, normal=False)
        foreign = self.driver('other-session', running=True, normal=False, evidence=[])
        session.cleanup_runs(self.receipt)
        second.child.wait.assert_called_once_with(timeout=15)
        foreign.child.stdin.write.assert_not_called()
        self.assertEqual(len(self.receipt['interrupted_cleanup']), 2)
        self.assertIn('error', self.receipt['interrupted_cleanup'][0])
        self.assertTrue(all(not row['normal_app_exit_claimed'] for row in self.receipt['interrupted_cleanup']))

    def test_out_of_order_normal_close_updates_its_own_record(self):
        older = session.NativeM1.__new__(session.NativeM1)
        older.child = Mock()
        older.child.wait.return_value = 0
        older.send = Mock(return_value=dict(app_returncode=0))
        older.identities = {}
        older.record = dict(run='older')
        later = dict(run='later')
        older.evidence = [older.record, later]
        older.close()
        self.assertTrue(older.record['owned_processes_absent'])
        self.assertNotIn('normal_app_exit', later)

    def test_failed_readiness_still_registers_child_for_cleanup(self):
        child = Mock()
        child.poll.return_value = None
        child.wait.return_value = 0
        with patch.object(session.subprocess, 'Popen', return_value=child), patch.object(Path, 'open'), patch.object(session.NativeM1, 'read', side_effect=RuntimeError('readiness failed')):
            with self.assertRaisesRegex(RuntimeError, 'readiness failed'):
                session.NativeM1(Path('/workspace/synthetic-exe'), Path('/workspace/synthetic-run'), [], self.receipt['runs'])
        session.cleanup_runs(self.receipt)
        child.wait.assert_called_once_with(timeout=15)
        self.assertEqual(self.receipt['status'], 'active')


if __name__ == '__main__':
    unittest.main()
