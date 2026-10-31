import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_test_plan import commands_for, load_workspace, select_tests, target_for

ROOT = Path(__file__).resolve().parents[2]


class TestPlanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workspace = load_workspace(ROOT)

    def plan(self, *paths):
        return select_tests(list(paths), self.workspace)

    def test_web_server_source_runs_only_web_server(self):
        plan = self.plan('crates/services/web-server/src/lib.rs')
        self.assertEqual(plan['selections'], {'web-server': []})
        self.assertTrue(plan['registry'])
        self.assertTrue(plan['backend_contracts'])
        self.assertFalse(plan['validator'])

    def test_core_source_runs_consumers_but_not_its_dependencies(self):
        plan = self.plan('crates/libs/lib-core/src/model.rs')
        self.assertEqual(set(plan['selections']), {
            'lib-core', 'lib-rest-core', 'lib-web', 'xml', 'validator', 'web-server', 'terminology-loader'})
        self.assertTrue(plan['backend_contracts'])
        self.assertTrue(plan['validator'])

    def test_validator_source_also_checks_its_http_consumer(self):
        self.assertEqual(self.plan('crates/libs/validator/src/lib.rs')['selections'],
                         {'validator': [], 'web-server': []})
        tools = self.plan('crates/libs/validator/tools/check_rule_tests.py')
        self.assertEqual(tools['selections'], {'validator': []})
        self.assertFalse(tools['backend'])
        self.assertFalse(tools['registry'])

    def test_leaf_tool_does_not_run_library_or_http_tests(self):
        plan = self.plan('crates/tools/gen-key/src/main.rs')
        self.assertEqual(plan['selections'], {'gen-key': []})
        self.assertFalse(plan['backend_contracts'])
        self.assertFalse(plan['xml_assets'])

    def test_test_only_changes_do_not_propagate_to_consumers(self):
        plan = self.plan('crates/libs/lib-core/tests/authorization/storage.rs',
                         'crates/libs/lib-core/tests/presave/audit.rs')
        self.assertEqual(plan['selections'], {'lib-core': ['authorization', 'section_presave']})
        self.assertFalse(plan['validator'])
        self.assertFalse(plan['registry'])
        self.assertFalse(plan['xml_assets'])

    def test_shared_test_helpers_expand_only_their_own_crate(self):
        for package in ('lib-core', 'web-server', 'xml'):
            root = self.workspace[package]['root']
            for paths in ([root + 'tests/common/mod.rs'],
                          [root + 'tests/common/mod.rs', root + 'tests/unknown.rs']):
                self.assertEqual(self.plan(*paths)['selections'], {package: []})
        self.assertEqual(self.plan('crates/services/web-server/tests/validation/validation_common.rs')['selections'],
                         {'web-server': ['api']})

    def test_every_cargo_target_and_nested_family_resolves(self):
        for package, data in self.workspace.items():
            for target, path in data['targets'].items():
                self.assertEqual(target_for(package, path, self.workspace), target)
        cases = {
            'web-server': {'api/case_contract/support.rs': 'api', 'authz/rbac_users/helpers.rs': 'authz',
                           'api/terminology_web.rs': 'smoke', 'xml/roundtrip_profiles_web.rs': 'smoke'},
            'lib-core': {'authorization/snapshot.rs': 'authorization', 'presave/audit.rs': 'section_presave',
                         'ui/authorization/permit_cannot_escape_transaction.stderr': 'authorization_kernel',
                         'snapshots/authorization_catalog.sha256': 'authorization_contract_snapshot',
                         'terminology/terminology_queries.rs': 'terminology'},
            'xml': {'import/common.rs': 'import', 'xml/xml_patch_c.rs': 'xml'},
        }
        for package, files in cases.items():
            for path, target in files.items():
                self.assertEqual(target_for(package, 'tests/' + path, self.workspace), target)

    def test_api_execution_retains_contract_and_registry_gates(self):
        for path in ('api.rs', 'api/case_contract/save.rs'):
            plan = self.plan('crates/services/web-server/tests/' + path)
            self.assertEqual(plan['selections'], {'web-server': ['api']})
            self.assertTrue(plan['registry'])
            self.assertTrue(plan['backend_contracts'])
        plan = self.plan('crates/services/web-server/tests/authz/authorization_snapshot.rs')
        self.assertEqual(plan['selections'], {'web-server': ['authz']})
        self.assertFalse(plan['registry'])
        self.assertFalse(plan['backend_contracts'])

    def test_mixed_source_and_test_changes_widen_the_affected_crate(self):
        paths = ['crates/libs/lib-core/tests/authorization/audit.rs', 'crates/libs/lib-core/src/model.rs']
        for ordered in (paths, paths[::-1]):
            plan = self.plan(*ordered)
            self.assertEqual(plan['selections']['lib-core'], [])
            self.assertIn('web-server', plan['selections'])

    def test_removed_target_runs_remaining_crate_tests(self):
        workspace = {name: {**data, 'targets': dict(data['targets'])} for name, data in self.workspace.items()}
        del workspace['lib-core']['targets']['section_presave']
        for path in ('section_presave.rs', 'presave/audit.rs'):
            plan = select_tests(['crates/libs/lib-core/tests/' + path], workspace)
            self.assertEqual(plan['selections'], {'lib-core': []})

    def test_cross_crate_test_move_selects_both_owners(self):
        self.assertEqual(self.plan('crates/services/web-server/tests/authz/authorization_storage.rs',
                                   'crates/libs/lib-core/tests/authorization/storage.rs')['selections'],
                         {'lib-core': ['authorization'], 'web-server': ['authz']})

    def test_documentation_and_empty_diff_skip_tests(self):
        for paths in ([], ['README.md', 'AGENTS.md', 'docs/design.md', 'registry/README.md',
                          'crates/libs/lib-core/README.md', 'crates/services/web-server/AGENTS.md']):
            plan = self.plan(*paths)
            self.assertEqual(plan['selections'], {})
            for flag in ('validator', 'backend', 'registry', 'backend_contracts', 'rust'):
                self.assertFalse(plan[flag])

    def test_shared_inputs_select_consumers_and_unknown_inputs_are_explicit(self):
        db = self.plan('db/migrations/new.sql')
        self.assertEqual(db['selections'], self.plan('crates/libs/lib-core/src/model.rs')['selections'])
        registry = self.plan('registry/editor-contracts/ci.json')
        self.assertIn('input-contracts', registry['selections'])
        self.assertTrue(registry['registry'])
        self.assertTrue(registry['validator'])
        xml = self.plan('docs/exporter/fda/fixture.xml')
        self.assertEqual(set(xml['selections']), {'xml', 'lib-rest-core', 'lib-web', 'web-server'})
        for path in ('Cargo.lock', 'Cargo.toml', '.github/workflows/ci.yml',
                     'scripts/ci_test_plan.py', 'scripts/test-isolated-db.sh', 'unknown/shared-input'):
            plan = self.plan(path)
            self.assertEqual(set(plan['selections']), set(self.workspace))
            self.assertEqual(plan['full_reasons'], [path])

    def test_commands_are_per_crate_locked_and_serial_with_separate_rule_gate(self):
        selection = {'web-server': ['api', 'authz'], 'lib-core': ['authorization'], 'validator': []}
        self.assertEqual(commands_for(selection, self.workspace), [
            ['scripts/test-isolated-db.sh', '--locked', '-p', 'lib-core', '-j1', '--test', 'authorization',
             '--', '--test-threads=1', '--color', 'never'],
            ['scripts/test-isolated-db.sh', '--locked', '-p', 'web-server', '-j1', '--test', 'api', '--test', 'authz',
             '--', '--test-threads=1', '--color', 'never'],
        ])
        for kind in ('fmt', 'clippy'):
            command = commands_for({'web-server': []}, self.workspace, kind)[0]
            self.assertIn('web-server', command)
            self.assertNotIn('--workspace', command)
            self.assertNotIn('--all', command)
        for invalid in ({}, [], {'$(touch bad)': []}, {'web-server': ['missing']},
                        {'web-server': ['api', 'api']}, {'web-server': [None]}, {'web-server': 'api'}):
            with self.assertRaises(ValueError):
                commands_for(invalid, self.workspace)

    def test_push_and_pr_share_one_plan_and_discover_new_workspace_crates(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=repo, stderr=subprocess.DEVNULL).decode().strip()
            git('init', '-q')
            git('config', 'user.name', 'Selector test')
            git('config', 'user.email', 'selector@example.test')
            git('config', 'commit.gpgsign', 'false')
            git('config', 'core.hooksPath', '/dev/null')
            (repo / 'Cargo.toml').write_text('[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n')
            crate = repo / 'crates/alpha'
            (crate / 'src').mkdir(parents=True)
            (crate / 'tests').mkdir()
            (crate / 'Cargo.toml').write_text('[package]\nname = "alpha"\nversion = "0.1.0"\nedition = "2021"\n')
            (crate / 'src/lib.rs').write_text('')
            (crate / 'tests/contract.rs').write_text('#[test] fn contract() { assert_eq!(2 + 2, 4); }\n')
            git('add', '.')
            git('commit', '-qm', 'initial workspace')
            base = git('rev-parse', 'HEAD')
            (crate / 'tests/contract.rs').write_text('#[test] fn contract() { assert_eq!(3 + 3, 6); }\n')
            git('add', '.')
            git('commit', '-qm', 'test change')
            plans = []
            for event in ('push', 'pull_request'):
                output_file = repo / (event + '.output')
                output = subprocess.check_output([
                    sys.executable, str(ROOT / 'scripts/ci_test_plan.py'), 'plan', '--event', event,
                    '--base', base, '--head', 'HEAD', '--github-output', str(output_file)], cwd=repo)
                plans.append(json.loads(output))
                self.assertIn('selections={"alpha": ["contract"]}', output_file.read_text())
            for plan in plans:
                self.assertEqual(plan['selections'], {'alpha': ['contract']})
                self.assertFalse(plan['registry'])
            self.assertEqual(plans[0]['changed_paths'], plans[1]['changed_paths'])


if __name__ == '__main__':
    unittest.main()
