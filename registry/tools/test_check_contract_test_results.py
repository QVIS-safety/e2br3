import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from check_contract_test_results import (
    backend_expected_tests,
    check_backend_results,
    check_frontend_results,
    expected_tests,
    require_backend_projection_parity,
)
from generate_editor_contract_web_tests import ACTIVE_PAGES, expected_editor_tests


class ContractTestResultsTests(unittest.TestCase):
    def setUp(self):
        self.inventory = {'fields': [
            {'identity': 'CI|ICH|C.1.3|reportType', 'page': 'CI', 'code': 'C.1.3',
             'authority': 'ICH', 'frontendPath': 'reportType',
             'contractMetadata': {'constraint': {'status': 'verified'}}},
            {'identity': 'RE|ICH|receiver|reviewReceivers[].receiver', 'page': 'RE',
             'code': 'receiver', 'authority': 'ICH', 'frontendPath': 'reviewReceivers[].receiver',
             'contractMetadata': None, 'automatedTests': {
                 'backend': [{'file': 'tests/api_cases/re.rs', 'test': 'case_contract_web::saves_receiver'}],
                 'frontend': [{'file': '__tests__/review.test.tsx', 'test': 'saves receiver'}]}},
        ]}
        self.log = ('running 2 tests\n'
                    'test case_editor_contract_web::ci::fields::c_1_3 ... ok\n'
                    'test case_contract_web::saves_receiver ... ok\n'
                    'test result: ok. 2 passed; 0 failed; 0 ignored;\n')
        self.frontend_root = Path('/frontend')
        self.report = {'success': True, 'wasInterrupted': False, 'testResults': [
            {'name': '/frontend/__tests__/case-save/editor-field-contract.generated.test.ts',
             'status': 'passed', 'assertionResults': [
                 {'title': 'CI C.1.3 emits canonical rows payload', 'status': 'passed'}]},
            {'name': '/frontend/__tests__/case-editor/editor-contract-snapshot.test.ts',
             'status': 'passed', 'assertionResults': [
                 {'title': 'CI ICH C.1.3 reportType rejects invalid input', 'status': 'passed'}]},
            {'name': '/frontend/__tests__/review.test.tsx', 'status': 'passed',
             'assertionResults': [{'title': 'saves receiver', 'status': 'passed'}]},
        ]}

    def test_checks_generated_and_explicit_tests_on_both_sides(self):
        self.assertEqual(2, check_backend_results(self.inventory, self.log))
        self.assertEqual(3, check_frontend_results(self.inventory, self.report, self.frontend_root))

    def test_backend_rejects_missing_ignored_failed_duplicate_and_incomplete_results(self):
        receiver = 'test case_contract_web::saves_receiver ... ok\n'
        for log in [self.log.replace(receiver, ''),
                    self.log.replace(receiver, receiver.replace('ok', 'ignored')),
                    self.log.replace(receiver, receiver.replace('ok', 'FAILED')),
                    self.log.replace(receiver, receiver * 2),
                    self.log.replace(receiver, receiver.replace('case_contract_web', 'wrong_module')),
                    self.log.split('test result:')[0],
                    self.log.replace('test result: ok.', 'test result: FAILED.'), '']:
            with self.subTest(log=log), self.assertRaises(ValueError):
                check_backend_results(self.inventory, log)

    def test_frontend_rejects_skipped_todo_missing_duplicate_and_wrong_file_results(self):
        for index in range(3):
            for status in ['pending', 'todo', 'failed', 'disabled']:
                report = copy.deepcopy(self.report)
                report['testResults'][index]['assertionResults'][0]['status'] = status
                with self.subTest(index=index, status=status), self.assertRaises(ValueError):
                    check_frontend_results(self.inventory, report, self.frontend_root)
            for change in ['missing', 'duplicate', 'wrong_file']:
                report = copy.deepcopy(self.report)
                suite = report['testResults'][index]
                if change == 'missing':
                    suite['assertionResults'].clear()
                elif change == 'duplicate':
                    suite['assertionResults'] *= 2
                else:
                    suite['name'] = '/frontend/wrong.test.ts'
                with self.subTest(index=index, change=change), self.assertRaises(ValueError):
                    check_frontend_results(self.inventory, report, self.frontend_root)

    def test_frontend_rejects_failed_interrupted_or_outside_repository_reports(self):
        for key, value in [('success', False), ('wasInterrupted', True)]:
            report = copy.deepcopy(self.report)
            report[key] = value
            with self.assertRaises(ValueError):
                check_frontend_results(self.inventory, report, self.frontend_root)
        for key, value in [('name', '/outside/review.test.tsx'), ('status', 'failed')]:
            report = copy.deepcopy(self.report)
            report['testResults'][0][key] = value
            with self.assertRaises(ValueError):
                check_frontend_results(self.inventory, report, self.frontend_root)

    def test_expected_tests_reject_empty_duplicate_and_unqualified_references(self):
        for fields in [[], self.inventory['fields'] * 2]:
            with self.assertRaises(ValueError):
                expected_tests({'fields': fields}, 'backend')
        for references in [[], [{'file': 'tests/api_cases/re.rs', 'test': 'saves_receiver'}]]:
            inventory = copy.deepcopy(self.inventory)
            inventory['fields'][1]['automatedTests']['backend'] = references
            with self.assertRaises(ValueError):
                expected_tests(inventory, 'backend')

    def test_shared_explicit_tests_are_required_once(self):
        field = copy.deepcopy(self.inventory['fields'][1])
        field['identity'] += '.second'
        self.inventory['fields'].append(field)
        self.assertEqual(2, check_backend_results(self.inventory, self.log))
        self.assertEqual(3, check_frontend_results(self.inventory, self.report, self.frontend_root))

    def test_selected_targets_require_all_of_their_contracts_without_unrelated_tests(self):
        for targets, expected_name in [({'api_editor'}, 'case_editor_contract_web::ci::fields::c_1_3'),
                                       ({'api_cases'}, 'case_contract_web::saves_receiver')]:
            expected = expected_tests(self.inventory, 'backend', targets)
            self.assertEqual(expected, {expected_name})
            log = f'test {expected_name} ... ok\ntest result: ok. 1 passed; 0 failed\n'
            self.assertEqual(1, check_backend_results(self.inventory, log, targets))
            for invalid in ('', log.replace('... ok', '... ignored'), log + log):
                with self.assertRaises(ValueError):
                    check_backend_results(self.inventory, invalid, targets)
        self.assertEqual(2, check_backend_results(self.inventory, self.log, {'api_editor', 'api_cases'}))
        with self.assertRaises(ValueError):
            check_backend_results(self.inventory, self.log, {'api_presave'})

    def test_non_ui_contract_checks_are_required_and_scope_keeps_fail_closed_gate(self):
        field = 'case_editor_contract_web::ci::fields::c_1_3'
        row = 'case_editor_contract_web::ci::rows::identifiers'
        catalog = {field, row}
        with self.assertRaises(ValueError):
            check_backend_results(self.inventory, self.log, {'api_editor'}, catalog)
        log = f'test {row} ... ok\ntest result: ok. 1 passed; 0 failed\n'
        filters = ['case_editor_contract_web::ci::rows::']
        self.assertEqual(1, check_backend_results(self.inventory, log, {'api_editor'}, catalog, filters))
        for invalid in ('', log + log, log.replace('... ok', '... ignored')):
            with self.assertRaises(ValueError):
                check_backend_results(self.inventory, invalid, {'api_editor'}, catalog, filters)

    def test_selected_targets_keep_each_explicit_reference_in_its_own_target(self):
        inventory = copy.deepcopy(self.inventory)
        inventory['fields'][1]['automatedTests']['backend'].append({
            'file': 'crates/services/web-server/tests/api_import/import_contract_web.rs',
            'test': 'import_contract_web::persists_product',
        })
        self.assertEqual({'import_contract_web::persists_product'},
                         expected_tests(inventory, 'backend', {'api_import'}))
        self.assertEqual(3, len(expected_tests(inventory, 'backend')))

    def test_real_registry_backend_projection_is_the_union_of_its_sources(self):
        registry = Path(__file__).resolve().parents[1]
        editor = expected_editor_tests(registry)
        cases = backend_expected_tests(registry, {'api_cases'})
        imports = backend_expected_tests(registry, {'api_import'})
        self.assertEqual(editor, backend_expected_tests(registry, {'api_editor'}))
        self.assertEqual(editor | cases | imports, backend_expected_tests(registry))
        self.assertIn(
            'case_contract_web::review_receivers::editor_shell_reads_saved_review_receivers_as_an_array',
            cases,
        )
        self.assertIn(
            'import_contract_web::test_import_selected_product_links_sender_without_overwriting_xml_sender',
            imports,
        )

    def test_registry_projection_requires_real_qualified_targeted_test_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            registry = root / 'registry'
            section = registry / 'sections/contract.json'
            section.parent.mkdir(parents=True)
            (registry / 'index.json').write_text(json.dumps({
                'sections': ['sections/contract.json'],
            }))
            source = root / 'tests/api_cases/contract.rs'
            source.parent.mkdir(parents=True)
            source.write_text('#[tokio::test]\nasync fn saves_receiver() {}\n')

            def write_reference(file='tests/api_cases/contract.rs',
                                test='contract_web::saves_receiver'):
                section.write_text(json.dumps([{
                    'automatedTests': {'backend': [{'file': file, 'test': test}]},
                }]))

            write_reference()
            self.assertEqual(
                {'contract_web::saves_receiver'},
                backend_expected_tests(registry, {'api_cases'}, set()),
            )
            for file, test, error in [
                ('tests/api_cases/missing.rs', 'contract_web::saves_receiver',
                 'backend test file missing'),
                ('tests/api_cases/contract.rs', 'saves_receiver',
                 'full Rust module path'),
                ('contract.rs', 'contract_web::saves_receiver',
                 'tests/<target>/<file> path'),
                ('tests/api_cases', 'contract_web::saves_receiver',
                 'tests/<target>/<file> path'),
                ('tests/api-cases/contract.rs', 'contract_web::saves_receiver',
                 'valid Cargo test target'),
                ('tests/api_cases/../api_import/contract.rs',
                 'contract_web::saves_receiver', 'repository-relative file path'),
                ('tests/api_cases/contract.rs', 'contract_web::renamed',
                 'test declaration missing'),
            ]:
                write_reference(file, test)
                with self.subTest(file=file, test=test), self.assertRaisesRegex(ValueError, error):
                    backend_expected_tests(registry, {'api_cases'}, set())
            for invalid in [None, {}, {'backend': None}]:
                section.write_text(json.dumps([{'automatedTests': invalid}]))
                with self.subTest(automatedTests=invalid), self.assertRaises(ValueError):
                    backend_expected_tests(registry, {'api_cases'}, set())

    def test_inventory_parity_checks_each_backend_target_not_only_the_union(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            registry = root / 'registry'
            section = registry / 'sections/contract.json'
            section.parent.mkdir(parents=True)
            (registry / 'index.json').write_text(json.dumps({
                'sections': ['sections/contract.json'],
            }))
            references = [
                {'file': 'tests/api_cases/cases.rs', 'test': 'case_contract_web::persists_case'},
                {'file': 'tests/api_cases/review.rs', 'test': 'case_contract_web::persists_review'},
                {'file': 'tests/api_import/import.rs', 'test': 'import_contract_web::persists_import'},
            ]
            for reference in references:
                source = root / reference['file']
                source.parent.mkdir(parents=True, exist_ok=True)
                name = reference['test'].rsplit('::', 1)[-1]
                source.write_text(f'#[test]\nfn {name}() {{}}\n')
            section.write_text(json.dumps([{
                'automatedTests': {'backend': references},
            }]))
            contracts = registry / 'editor-contracts'
            contracts.mkdir()
            for page in ACTIVE_PAGES:
                fields = ([{'code': 'C.1.3'}] if page == 'CI' else [])
                (contracts / f'{page.lower()}.json').write_text(json.dumps({
                    'pageId': page,
                    'fields': fields,
                    'fieldChecks': [],
                    'collections': [],
                    'relations': [],
                    'auditChecks': [],
                }))
            inventory = {'fields': [
                {'identity': 'CI|ICH|C.1.3|reportType', 'page': 'CI', 'code': 'C.1.3',
                 'authority': 'ICH', 'frontendPath': 'reportType', 'contractMetadata': None},
                {'identity': 'RE|ICH|receiver|reviewReceivers[].receiver', 'page': 'RE',
                 'code': 'receiver', 'authority': 'ICH',
                 'frontendPath': 'reviewReceivers[].receiver', 'contractMetadata': None,
                 'automatedTests': {'backend': copy.deepcopy(references)}},
            ]}
            require_backend_projection_parity(inventory, registry)
            inventory['fields'][1]['automatedTests']['backend'][0]['file'] = (
                'tests/api_import/cases.rs'
            )
            with self.assertRaisesRegex(ValueError, 'projection mismatch for api_cases'):
                require_backend_projection_parity(inventory, registry)


if __name__ == '__main__':
    unittest.main()
