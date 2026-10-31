#!/usr/bin/env python3
"""Require actual passing tests for canonical frontend and backend contracts."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

from generate_editor_contract_web_tests import test_name, expected_editor_execution_tests, normalize_editor_filters


BACKEND_CONTRACT_TARGETS = ('api_cases', 'api_editor', 'api_import')


def backend_reference(reference: dict, backend_root: Path | None = None) -> tuple[str, str]:
    if (not isinstance(reference, dict) or set(reference) != {'file', 'test'}
            or any(not isinstance(value, str) or not value.strip()
                   for value in reference.values())):
        raise ValueError('backend test references require nonempty file/test strings')
    name = reference['test']
    if not re.fullmatch(r'\w+(?:::\w+)+', name):
        raise ValueError('backend test references require the full Rust module path')
    reference_path = Path(reference['file'])
    parts = reference_path.parts
    if reference_path.is_absolute() or '..' in parts:
        raise ValueError('backend test references require a repository-relative file path')
    tests_index = parts.index('tests') if 'tests' in parts else -1
    if tests_index < 0 or tests_index + 2 >= len(parts):
        raise ValueError('backend test references require a tests/<target>/<file> path')
    target = parts[tests_index + 1]
    if not re.fullmatch(r'\w+', target):
        raise ValueError('backend test references require a valid Cargo test target')
    if backend_root is not None:
        root = backend_root.resolve()
        path = (root / reference_path).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            raise ValueError(f'backend test file missing or outside repository: {path}')
        declaration = re.escape(name.rsplit('::', 1)[-1])
        if not re.search(
            rf'(?m)^\s*#\[(?:tokio::)?test\]\s*(?:async\s+)?fn\s+{declaration}\s*\(',
            path.read_text(encoding='utf-8'),
        ):
            raise ValueError(f'backend test declaration missing: {path}: {name}')
    return target, name


def filter_editor_tests(expected: set[str], backend_targets: set[str] | None,
                        editor_filters: list[str] | None) -> set[str]:
    filters = normalize_editor_filters(editor_filters if editor_filters is not None else [])
    if filters:
        if backend_targets is None or 'api_editor' not in backend_targets:
            raise ValueError('editor filters require an explicitly selected api_editor target')
        return {name for name in expected if not name.startswith('case_editor_contract_web::')
                or any(name.startswith(prefix) for prefix in filters)}
    return expected


def expected_tests(inventory: dict, side: str, backend_targets: set[str] | None = None, editor_tests: set[str] | None = None, editor_filters: list[str] | None = None) -> set:
    fields = inventory['fields']
    identities = [field['identity'] for field in fields]
    if not fields or len(identities) != len(set(identities)):
        raise ValueError('canonical fields must be nonempty and unique')
    expected = set()
    for field in fields:
        references = field.get('automatedTests', {})
        if side in references:
            if not references[side]:
                raise ValueError(f'{field["identity"]}: empty {side} test references')
            for reference in references[side]:
                if side == 'backend':
                    target, name = backend_reference(reference)
                    if backend_targets is not None and target not in backend_targets:
                        continue
                    expected.add(name)
                else:
                    expected.add((reference['file'], reference['test']))
        elif side == 'backend':
            if backend_targets is None or 'api_editor' in backend_targets:
                expected.add(f'case_editor_contract_web::{field["page"].lower()}::fields::{test_name(field["code"])}')
        else:
            expected.add(('__tests__/case-save/editor-field-contract.generated.test.ts',
                          f'{field["page"]} {field["code"]} emits canonical rows payload'))
        contract = field['contractMetadata']
        if side == 'frontend' and contract and contract.get('constraint', {}).get('status') == 'verified':
            expected.add(('__tests__/case-editor/editor-contract-snapshot.test.ts',
                          f'{field["page"]} {field["authority"]} {field["code"]} '
                          f'{field["frontendPath"]} rejects invalid input'))
    if side == 'backend':
        if editor_tests is not None and (backend_targets is None or 'api_editor' in backend_targets):
            expected.update(editor_tests)
        expected = filter_editor_tests(expected, backend_targets, editor_filters)
    if side == 'backend' and not expected:
        raise ValueError('selected backend targets contain no canonical contract tests')
    return expected


def backend_expected_tests(registry_root: Path, backend_targets: set[str] | None = None,
                           editor_tests: set[str] | None = None,
                           editor_filters: list[str] | None = None) -> set[str]:
    registry_root = registry_root.resolve()
    index = json.loads((registry_root / 'index.json').read_text(encoding='utf-8'))
    sections = index.get('sections') if isinstance(index, dict) else None
    if (not isinstance(sections, list) or not sections
            or any(not isinstance(section, str) or not section for section in sections)
            or len(sections) != len(set(sections))):
        raise ValueError('registry index sections must be nonempty, unique file paths')
    expected = set()
    for section in sections:
        path = (registry_root / section).resolve()
        if not path.is_relative_to(registry_root) or not path.is_file():
            raise ValueError(f'registry section missing or outside registry: {path}')
        rows = json.loads(path.read_text(encoding='utf-8'))
        if not isinstance(rows, list) or not rows:
            raise ValueError(f'registry section must contain a nonempty row array: {path}')
        for row in rows:
            if not isinstance(row, dict):
                raise ValueError(f'registry section row must be an object: {path}')
            if 'automatedTests' not in row:
                continue
            tests = row['automatedTests']
            if (not isinstance(tests, dict) or not tests
                    or set(tests) - {'backend', 'frontend'}):
                raise ValueError('automatedTests must contain backend or frontend references')
            if 'backend' not in tests:
                continue
            references = tests['backend']
            if not isinstance(references, list) or not references:
                raise ValueError('automatedTests.backend must be a nonempty array')
            for reference in references:
                target, name = backend_reference(reference, registry_root.parent)
                if backend_targets is None or target in backend_targets:
                    expected.add(name)
    if editor_tests is None:
        editor_tests = expected_editor_execution_tests(registry_root)
    if backend_targets is None or 'api_editor' in backend_targets:
        expected.update(editor_tests)
    expected = filter_editor_tests(expected, backend_targets, editor_filters)
    if not expected:
        raise ValueError('selected backend targets contain no canonical contract tests')
    return expected


def require_backend_projection_parity(inventory: dict, registry_root: Path) -> None:
    editor_tests = expected_editor_execution_tests(registry_root)
    for target in (None, *(frozenset({name}) for name in BACKEND_CONTRACT_TARGETS)):
        inventory_expected = expected_tests(inventory, 'backend', target, editor_tests)
        registry_expected = backend_expected_tests(registry_root, target, editor_tests)
        if inventory_expected != registry_expected:
            label = 'all' if target is None else next(iter(target))
            raise ValueError(
                f'backend contract projection mismatch for {label}: '
                f'inventory_only={sorted(inventory_expected - registry_expected)}, '
                f'registry_only={sorted(registry_expected - inventory_expected)}'
            )


def require_passed(expected: set, results: dict, passed: str) -> None:
    missing = expected - results.keys()
    invalid = {name: results[name] for name in expected & results.keys()
               if results[name] != [passed]}
    if missing or invalid:
        raise ValueError(f'contract test execution mismatch: missing={sorted(missing)}, '
                         f'failed/skipped/duplicate={invalid}')


def check_backend_expected_results(expected: set[str], log: str) -> int:
    results, pending = {}, {}
    for line in log.splitlines():
        match = re.match(r'^test (\S+) \.\.\. (\S+)', line)
        if match:
            pending.setdefault(match[1], []).append(match[2])
        elif line.startswith('test result: '):
            if not line.startswith('test result: ok.'):
                raise ValueError(f'Cargo suite did not pass: {line}')
            for name, statuses in pending.items():
                results.setdefault(name, []).extend(statuses)
            pending.clear()
    if pending:
        raise ValueError('Cargo output ended before the test suite completed')
    require_passed(expected, results, 'ok')
    return len(expected)


def check_backend_results(inventory: dict, log: str, backend_targets: set[str] | None = None, editor_tests: set[str] | None = None, editor_filters: list[str] | None = None) -> int:
    expected = expected_tests(inventory, 'backend', backend_targets, editor_tests, editor_filters)
    return check_backend_expected_results(expected, log)


def check_frontend_results(inventory: dict, report: dict, frontend_root: Path) -> int:
    if report['success'] is not True or report['wasInterrupted'] is not False:
        raise ValueError('Jest run failed or was interrupted')
    expected = expected_tests(inventory, 'frontend')
    results = {}
    for suite in report['testResults']:
        filename = Path(suite['name']).resolve().relative_to(frontend_root.resolve()).as_posix()
        if suite['status'] != 'passed':
            raise ValueError(f'Jest suite did not pass: {filename}: {suite["status"]}')
        for assertion in suite['assertionResults']:
            key = (filename, assertion['title'])
            results.setdefault(key, []).append(assertion['status'])
    require_passed(expected, results, 'passed')
    return len(expected)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path)
    parser.add_argument('--registry-root', type=Path,
                        default=Path(__file__).resolve().parents[1])
    parser.add_argument('--backend-log', type=Path)
    parser.add_argument('--backend-selections', help='CI package-to-targets JSON; an empty target list means the whole crate')
    parser.add_argument('--editor-filters', default='[]')
    parser.add_argument('--frontend-results', type=Path)
    parser.add_argument('--frontend-root', type=Path)
    args = parser.parse_args()
    if args.backend_log is None and args.frontend_results is None:
        parser.error('provide --backend-log or --frontend-results')
    if args.frontend_results is not None and (args.frontend_root is None or args.inventory is None):
        parser.error('--frontend-results requires --inventory and --frontend-root')
    if args.backend_selections is not None and args.backend_log is None:
        parser.error('--backend-selections requires --backend-log')
    backend_targets = None
    if args.backend_selections is not None:
        selections = json.loads(args.backend_selections)
        if not isinstance(selections, dict) or 'web-server' not in selections:
            raise ValueError('backend selections must include web-server')
        targets = selections['web-server']
        if (not isinstance(targets, list) or
                any(not isinstance(target, str) or not re.fullmatch(r'\w+', target) for target in targets) or
                len(targets) != len(set(targets))):
            raise ValueError('backend selections must contain unique Cargo target names')
        backend_targets = set(targets) if targets else None
    inventory = json.loads(args.inventory.read_text()) if args.inventory is not None else None
    result = {}
    if inventory is not None:
        require_backend_projection_parity(inventory, args.registry_root)
        result['canonicalFields'] = len(inventory['fields'])
    if args.backend_log is not None:
        expected = backend_expected_tests(args.registry_root, backend_targets,
                                          editor_filters=json.loads(args.editor_filters))
        result['backendTestsRequired'] = len(expected)
        result['backendTestsPassed'] = check_backend_expected_results(
            expected, args.backend_log.read_text())
        result['backendTargets'] = sorted(backend_targets) if backend_targets is not None else 'all'
    if args.frontend_results is not None:
        result['frontendTestsPassed'] = check_frontend_results(
            inventory, json.loads(args.frontend_results.read_text()), args.frontend_root)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
