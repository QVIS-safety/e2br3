import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_test_plan import ROOTS, TARGETS, changed_paths, select_tests, target_commands, target_for


class TestPlanTests(unittest.TestCase):
    def test_push_always_requires_full_backend_contract_execution(self):
        for paths in ([], ['README.md'], [ROOTS['web-server'] + 'authz.rs']):
            self.assertEqual(select_tests('push', paths),
                             {'mode': 'full', 'targets': [], 'backend_contracts': True})

    def test_only_documentation_skips_database_tests(self):
        self.assertEqual(select_tests('pull_request', ['README.md', 'docs/design.md']),
                         {'mode': 'none', 'targets': [], 'backend_contracts': False})

    def test_shared_production_and_unknown_paths_require_full_tests(self):
        paths = ['Cargo.lock', 'Cargo.toml', '.github/workflows/ci.yml', 'db/migrations/new.sql',
                 'registry/editor-contracts/ci.json', 'docs/exporter/fda/fixture.xml',
                 'crates/libs/xml/src/lib.rs', 'scripts/test-isolated-db.sh',
                 ROOTS['web-server'] + 'common/mod.rs', ROOTS['web-server'] + 'validation/validation_common.rs',
                 ROOTS['lib-core'] + 'common/mod.rs', ROOTS['xml'] + 'new_target.rs']
        for path in paths:
            with self.subTest(path=path):
                self.assertEqual(select_tests('pull_request', ['README.md', path])['mode'], 'full')
                self.assertEqual(select_tests('pull_request', [ROOTS['web-server'] + 'api.rs', path])['mode'], 'full')
        self.assertEqual(select_tests('pull_request', [])['mode'], 'full')
        with self.assertRaises(ValueError):
            select_tests('unexpected', ['README.md'])

    def test_whole_targets_and_nested_helpers_map_to_their_owners(self):
        for package, targets in TARGETS.items():
            for target in targets:
                self.assertEqual(target_for(ROOTS[package] + target + '.rs'), f'{package}:{target}')
        cases = {
            'web-server': {'api/case_contract/support.rs': 'api', 'api/case_editor_contract/generated/ci_fields.rs': 'api',
                           'authz/rbac_users/helpers.rs': 'authz', 'api/terminology_web.rs': 'smoke',
                           'xml/roundtrip_profiles_web.rs': 'smoke'},
            'lib-core': {'ui/authorization/permit_cannot_escape_transaction.stderr': 'authorization_kernel',
                         'snapshots/authorization_catalog.sha256': 'authorization_contract_snapshot',
                         'terminology/terminology_queries.rs': 'terminology'},
            'xml': {'import/common.rs': 'import', 'xml/xml_patch_c.rs': 'xml'},
        }
        for package, files in cases.items():
            for path, target in files.items():
                self.assertEqual(target_for(ROOTS[package] + path), f'{package}:{target}')

    def test_api_executes_all_contract_tests_and_other_targets_do_not_claim_them(self):
        api = select_tests('pull_request', [ROOTS['web-server'] + 'api/case_contract/save.rs'])
        self.assertEqual(api, {'mode': 'targeted', 'targets': ['web-server:api'], 'backend_contracts': True})
        authz = select_tests('pull_request', [ROOTS['web-server'] + 'authz/rls_web.rs'])
        self.assertFalse(authz['backend_contracts'])
        self.assertEqual(authz['targets'], ['web-server:authz'])

    def test_removed_or_renamed_target_runs_remaining_workspace_tests(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            old = ROOTS['lib-core'] + 'terminology.rs'
            new = ROOTS['lib-core'] + 'authorization_contract_snapshot.rs'
            (root / new).parent.mkdir(parents=True)
            (root / new).write_text('// isolated target-selection fixture\n')
            for paths in ([old], [old, new]):
                self.assertEqual(select_tests('pull_request', paths, root)['mode'], 'full')
            self.assertEqual(select_tests('pull_request', [new], root)['mode'], 'targeted')

    def test_commands_group_targets_and_preserve_isolation_serialization(self):
        targets = ['web-server:authz', 'lib-core:authorization_kernel', 'web-server:api']
        commands = target_commands(targets)
        self.assertEqual(commands, [
            ['scripts/test-isolated-db.sh', '-p', 'web-server', '-j1', '--test', 'api', '--test', 'authz',
             '--', '--test-threads=1', '--color', 'never'],
            ['scripts/test-isolated-db.sh', '-p', 'lib-core', '-j1', '--test', 'authorization_kernel',
             '--', '--test-threads=1', '--color', 'never'],
        ])
        for invalid in ([], ['web-server:api'] * 2, ['$(touch bad)'], ['web-server:new'], {}, [None]):
            with self.assertRaises(ValueError):
                target_commands(invalid)

    def test_real_git_diff_accounts_for_renames_deletions_and_invalid_revisions(self):
        script = Path(__file__).resolve().parents[1] / 'ci_test_plan.py'
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=repo, stderr=subprocess.DEVNULL).decode().strip()
            git('init', '-q')
            git('config', 'user.name', 'Selector test')
            git('config', 'user.email', 'selector@example.test')
            original = ROOTS['web-server'] + 'common/mod.rs'
            destination = ROOTS['web-server'] + 'api/helpers.rs'
            (repo / original).parent.mkdir(parents=True)
            (repo / original).write_text('deterministic diff fixture\n')
            (repo / 'README.md').write_text('documentation\n')
            git('add', '.')
            git('commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD')
            (repo / destination).parent.mkdir(parents=True)
            git('mv', original, destination)
            git('rm', 'README.md')
            git('commit', '-qm', 'move shared helper')
            command = [sys.executable, str(script), 'plan', '--event', 'pull_request', '--base', base, '--head', 'HEAD']
            plan = json.loads(subprocess.check_output(command, cwd=repo))
            self.assertEqual(set(plan['changed_paths']), {original, destination, 'README.md'})
            self.assertEqual(plan['mode'], 'full')
            for invalid in ('missing-revision', ''):
                command[-3] = invalid
                result = subprocess.run(command, cwd=repo, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, b'')


if __name__ == '__main__':
    unittest.main()
