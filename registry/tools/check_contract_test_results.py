#!/usr/bin/env python3
"""Require actual passing tests for every field in the canonical UI inventory."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

from generate_editor_contract_web_tests import test_name, expected_editor_tests, normalize_editor_filters


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
                    if not re.fullmatch(r'\w+(?:::\w+)+', reference['test']):
                        raise ValueError('backend test references require the full Rust module path')
                    if backend_targets is not None:
                        parts = Path(reference['file']).parts
                        if 'tests' not in parts or parts.index('tests') + 1 >= len(parts):
                            raise ValueError('backend test references require a tests/<target>/ path')
                        if parts[parts.index('tests') + 1] not in backend_targets:
                            continue
                    expected.add(reference['test'])
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
        filters = normalize_editor_filters(editor_filters if editor_filters is not None else [])
        if filters:
            if backend_targets is None or 'api_editor' not in backend_targets:
                raise ValueError('editor filters require an explicitly selected api_editor target')
            expected = {name for name in expected if not name.startswith('case_editor_contract_web::')
                        or any(name.startswith(prefix) for prefix in filters)}
    if side == 'backend' and not expected:
        raise ValueError('selected backend targets contain no canonical contract tests')
    return expected


def require_passed(expected: set, results: dict, passed: str) -> None:
    missing = expected - results.keys()
    invalid = {name: results[name] for name in expected & results.keys()
               if results[name] != [passed]}
    if missing or invalid:
        raise ValueError(f'contract test execution mismatch: missing={sorted(missing)}, '
                         f'failed/skipped/duplicate={invalid}')


def check_backend_results(inventory: dict, log: str, backend_targets: set[str] | None = None, editor_tests: set[str] | None = None, editor_filters: list[str] | None = None) -> int:
    expected = expected_tests(inventory, 'backend', backend_targets, editor_tests, editor_filters)
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
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--backend-log', type=Path)
    parser.add_argument('--backend-selections', help='CI package-to-targets JSON; an empty target list means the whole crate')
    parser.add_argument('--editor-filters', default='[]')
    parser.add_argument('--frontend-results', type=Path)
    parser.add_argument('--frontend-root', type=Path)
    args = parser.parse_args()
    if args.backend_log is None and args.frontend_results is None:
        parser.error('provide --backend-log or --frontend-results')
    if args.frontend_results is not None and args.frontend_root is None:
        parser.error('--frontend-results requires --frontend-root')
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
    inventory = json.loads(args.inventory.read_text())
    result = {'canonicalFields': len(inventory['fields'])}
    if args.backend_log is not None:
        result['backendTestsPassed'] = check_backend_results(inventory, args.backend_log.read_text(), backend_targets,
            expected_editor_tests(Path(__file__).resolve().parents[1]), json.loads(args.editor_filters))
        result['backendTargets'] = sorted(backend_targets) if backend_targets is not None else 'all'
    if args.frontend_results is not None:
        result['frontendTestsPassed'] = check_frontend_results(
            inventory, json.loads(args.frontend_results.read_text()), args.frontend_root)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
