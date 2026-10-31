#!/usr/bin/env python3
"""Run changed crates and their consumers; test-only edits stay in their owning targets."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "registry/tools"))
from generate_editor_contract_web_tests import ACTIVE_PAGES, LEGACY_READ_PAGES, load_contract, normalize_editor_filters, expected_editor_execution_tests

from ci_test_scope import changed_paths, event_revisions

# Nested modules whose directory name differs from their Cargo integration target.
FAMILIES = {
    'lib-core': {'ui/authorization/': 'authorization_kernel',
                 'snapshots/authorization_catalog.sha256': 'authorization_contract_snapshot',
                 'presave/': 'section_presave'},
}

CONTRACT_TARGETS = {'api_cases', 'api_editor', 'api_import'}
EDITOR_GROUPS = {'fields': 'fieldChecks', 'rows': 'collections',
                 'relations': 'relations', 'audit': 'auditChecks'}
REGISTRY_ROOT = Path(__file__).resolve().parents[1] / 'registry'


def xml_assets_required(selections: dict) -> bool:
    return 'xml' in selections or ('web-server' in selections and (
        not selections['web-server'] or bool({'api_import', 'api_submission', 'xml'} & set(selections['web-server']))))


def xml_assets_for_cargo_args(args: list[str]) -> bool:
    # Parse only Cargo arguments; libtest filters after -- do not select build targets.
    if args[:1] == ['--']:
        args = args[1:]
    if '--' in args:
        args = args[:args.index('--')]
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('-p', '--package', action='append')
    parser.add_argument('--test', action='append')
    parser.add_argument('--workspace', '--all', action='store_true')
    for option in ('--all-targets', '--tests', '--lib', '--bins', '--examples', '--benches', '--doc'):
        parser.add_argument(option, action='store_true')
    for option in ('--bin', '--example', '--bench'):
        parser.add_argument(option, action='append')
    parsed, _ = parser.parse_known_args(args)
    if parsed.workspace or not parsed.package:
        return True
    if any(not re.fullmatch(r'[\w-]+(?:@[\w.+-]+)?', package) for package in parsed.package):
        return True  # Package globs/URLs can include asset consumers.
    other_targets = any(value for key, value in vars(parsed).items() if key not in ('package', 'test', 'workspace'))
    targets = parsed.test if parsed.test and not other_targets else []
    return xml_assets_required({package.split('@')[0]: targets for package in parsed.package})


def load_workspace(root: Path) -> dict:
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--no-deps', '--locked', '--format-version', '1'], cwd=root))
    packages = [p for p in metadata['packages'] if p['id'] in metadata['workspace_members']]
    by_path = {str(Path(p['manifest_path']).parent.resolve()): p['name'] for p in packages}
    workspace = {}
    for package in packages:
        path = Path(package['manifest_path']).parent
        workspace[package['name']] = {
            'root': path.relative_to(root.resolve()).as_posix() + '/',
            'dependencies': {by_path[str(Path(d['path']).resolve())] for d in package['dependencies']
                             if 'path' in d and str(Path(d['path']).resolve()) in by_path},
            'targets': {t['name']: Path(t['src_path']).relative_to(path).as_posix()
                        for t in package['targets'] if 'test' in t['kind']},
        }
    if not workspace:
        raise ValueError('Cargo returned no workspace packages')
    return workspace


def target_for(package: str, relative: str, workspace: dict) -> str | None:
    targets = workspace[package]['targets']
    for name, source in targets.items():
        if relative == source:
            return name
    for family, target in FAMILIES.get(package, {}).items():
        path = 'tests/' + family
        if relative == path or (family.endswith('/') and relative.startswith(path)):
            return target if target in targets else None
    return next((name for name in targets if relative.startswith(f'tests/{name}/')), None)


def consumers(packages: set[str], workspace: dict) -> set[str]:
    affected = set(packages)
    while True:
        added = {name for name, data in workspace.items() if data['dependencies'] & affected}
        if added <= affected:
            return affected
        affected.update(added)


def select_tests(paths: list[str], workspace: dict) -> dict:
    selected = {}  # Empty target list means all tests in this crate.
    production = set()
    full_reasons = []
    editor_filters = []
    editor_all = False
    contract_inputs_changed = False
    for path in paths:
        if path == 'crates/services/web-server/tests/api_editor/API_SPEC.md':
            continue
        if path.startswith('crates/services/web-server/tests/api_specs/') and (
            Path(path).suffix in ('.md', '.tsv') or Path(path).name in (
                'api-contract-manifest.json', 'check_api_contract_manifest.py',
            )
        ):
            continue
        if Path(path).name in ('README.md', 'AGENTS.md', 'subagent.md') or (path.startswith('docs/') and path.endswith('.md')):
            continue
        if path.startswith(('registry/', 'scripts/case-editor-audit/')):
            contract_inputs_changed = True
        if path.startswith('registry/editor-contracts/') and Path(path).stem.upper() in ACTIVE_PAGES and path.endswith('.json'):
            selected.setdefault('web-server', ['api_editor'])
            if selected['web-server'] and 'api_editor' not in selected['web-server']:
                selected['web-server'].append('api_editor')
            editor_filters.append(f'case_editor_contract_web::{Path(path).stem}::')
            continue
        owner = next((name for name, data in workspace.items() if path.startswith(data['root'])), None)
        if owner is not None:
            relative = path[len(workspace[owner]['root']):]
            if relative.startswith('tests/'):
                target = target_for(owner, relative, workspace)
                if target == 'api_editor':
                    parts = Path(relative).parts
                    if len(parts) == 4 and parts[2] == 'pages' and parts[3].endswith('.generated.rs') and parts[3].split('.')[0].upper() in ACTIVE_PAGES:
                        editor_filters.append(f'case_editor_contract_web::{parts[3].split(".")[0]}::')
                    elif parts[2:] == ('fields', 'message_header.rs'):
                        editor_filters.append('case_editor_contract_web::n::')
                    elif (len(parts) == 4 and parts[2] == 'legacy_reads'
                          and Path(parts[3]).stem.upper() in LEGACY_READ_PAGES):
                        editor_filters.append(f'case_editor_contract_web::legacy_reads::{Path(parts[3]).stem}::')
                    elif len(parts) == 4 and parts[2] in EDITOR_GROUPS:
                        # shortcut: leaf verifiers have no cross-leaf helpers; widen selection if such dependencies are added.
                        category = parts[2]
                        verifier = f'{category}::{Path(parts[3]).stem}::'
                        pages = [page for page in ACTIVE_PAGES
                                 if any(check['verifier'].startswith(verifier)
                                        for check in load_contract(REGISTRY_ROOT, page)[EDITOR_GROUPS[category]])]
                        # Unmapped/deleted verifiers must widen coverage, never silently skip it.
                        if not pages:
                            editor_all = True
                        editor_filters.extend(f'case_editor_contract_web::{page.lower()}::{category}::' for page in pages)
                    else:
                        editor_all = True
                if target is None:
                    selected[owner] = []
                elif owner not in selected:
                    selected[owner] = [target]
                elif selected[owner] and target not in selected[owner]:
                    selected[owner].append(target)
            elif relative.startswith('tools/'):
                selected[owner] = []
            else:
                production.add(owner)
        elif path.startswith('db/'):
            production.add('lib-core')
        elif path.startswith(('registry/', 'scripts/case-editor-audit/')):
            production.update(('input-contracts', 'validator'))
        elif path.startswith(('docs/exporter/', 'docs/refs/instances/', 'deploy/ec2/schemas/')) or path == 'scripts/prepare-xml-assets.sh':
            production.add('xml')
        else:
            # Unclassified shared inputs must be visible and cannot silently skip tests.
            full_reasons.append(path)
    affected = set(workspace) if full_reasons else consumers(production, workspace)
    selected.update({name: [] for name in affected})
    selected = {name: sorted(targets) for name, targets in sorted(selected.items())}
    if editor_filters and not editor_all and selected.get('web-server'):
        registered = expected_editor_execution_tests(REGISTRY_ROOT)
        editor_filters = [prefix for prefix in editor_filters if any(name.startswith(prefix) for name in registered)]
        if not editor_filters:
            raise ValueError('changed editor verifier has no registered contract checks')
    backend_contracts = 'web-server' in selected and (
        not selected['web-server'] or bool(CONTRACT_TARGETS.intersection(selected['web-server'])))
    return {
        'selections': selected,
        'editor_filters': [] if editor_all or 'web-server' in affected or selected.get('web-server') == [] else normalize_editor_filters(editor_filters),
        'validator': 'validator' in selected,
        'backend': any(name != 'validator' for name in selected),
        'registry': backend_contracts and (contract_inputs_changed or 'web-server' in affected),
        'backend_contracts': backend_contracts,
        'rust': bool(selected),
        'xml_assets': xml_assets_required(selected),
        'full_reasons': full_reasons,
    }


def commands_for(selections: dict, workspace: dict, kind: str = 'tests', editor_filters: list[str] | None = None) -> list[list[str]]:
    if not isinstance(selections, dict) or not selections:
        raise ValueError('selections must be a nonempty package-to-targets object')
    for package, targets in selections.items():
        if package not in workspace or not isinstance(targets, list) or any(not isinstance(t, str) for t in targets):
            raise ValueError('unknown package or invalid target list')
        if len(set(targets)) != len(targets) or not set(targets) <= workspace[package]['targets'].keys():
            raise ValueError('unknown or duplicate integration targets')
    filters = normalize_editor_filters(editor_filters if editor_filters is not None else [])
    if filters and ('web-server' not in selections or 'api_editor' not in selections['web-server']):
        raise ValueError('editor filters require an explicitly selected api_editor target')
    package_args = [arg for package in sorted(selections) for arg in ('-p', package)]
    if kind == 'fmt':
        return [['cargo', 'fmt', *package_args, '--check']]
    if kind == 'clippy':
        args = ['cargo', 'clippy', '--locked', '--all-features']
        whole_crates = [arg for package, targets in sorted(selections.items()) if not targets for arg in ('-p', package)]
        commands = [args + ['--all-targets', *whole_crates]] if whole_crates else []
        for package, targets in sorted(selections.items()):
            if targets:
                commands.append(args + ['-p', package] + [arg for target in targets for arg in ('--test', target)])
        return commands
    if kind != 'tests':
        raise ValueError(f'unknown check: {kind}')
    commands = []
    for package, targets in sorted(selections.items()):
        if package == 'validator':  # Dedicated job also verifies all rule bindings.
            continue
        args = ['scripts/test-isolated-db.sh', '--locked', '-p', package, '-j1', '--no-fail-fast']
        if package == 'web-server' and filters:
            # libtest accepts one include filter; exclude the other page/category buckets in one DB run.
            prefixes = [f'case_editor_contract_web::{page.lower()}::{category}::'
                        for page in ACTIVE_PAGES for category in EDITOR_GROUPS]
            prefixes += [f'case_editor_contract_web::legacy_reads::{page.lower()}::'
                         for page in LEGACY_READ_PAGES]
            skips = [arg for prefix in prefixes if not any(prefix.startswith(f) for f in filters)
                     for arg in ('--skip', prefix)]
            commands.append(args + ['--test', 'api_editor', 'case_editor_contract_web::',
                                    '--', '--test-threads=1', '--color', 'never', *skips])
            targets = [target for target in targets if target != 'api_editor']
            if not targets:
                continue
        for target in targets:
            args.extend(['--test', target])
        commands.append(args + ['--', '--test-threads=1', '--color', 'never'])
    return commands


def run_commands(commands: list[list[str]]) -> int:
    failed = 0
    for command in commands:
        print('Running:', ' '.join(command), flush=True)
        if subprocess.run(command).returncode != 0:
            failed += 1
    if failed:
        print(f'{failed} selected check command(s) failed', file=sys.stderr)
    return 1 if failed else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest='command', required=True)
    plan_parser = subparsers.add_parser('plan')
    plan_parser.add_argument('--event', choices=('push', 'pull_request'))
    plan_parser.add_argument('--base')
    plan_parser.add_argument('--head')
    plan_parser.add_argument('--github-output', type=Path)
    run_parser = subparsers.add_parser('run')
    run_parser.add_argument('--selections', required=True)
    run_parser.add_argument('--editor-filters', default='[]')
    run_parser.add_argument('--kind', choices=('tests', 'fmt', 'clippy'), default='tests')
    assets_parser = subparsers.add_parser('xml-assets', help='preview asset requirements for isolated-runner Cargo arguments')
    assets_parser.add_argument('cargo_args', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command == 'xml-assets':
        print(json.dumps(xml_assets_for_cargo_args(args.cargo_args)))
        return
    root = Path.cwd()
    workspace = load_workspace(root)
    if args.command == 'run':
        raise SystemExit(run_commands(commands_for(json.loads(args.selections), workspace, args.kind, json.loads(args.editor_filters))))
    event, base, head = (args.event, args.base, args.head) if args.event else event_revisions()
    paths = changed_paths(base, head, event == 'pull_request', root)
    plan = select_tests(paths, workspace)
    print(json.dumps({'event': event, 'base': base, 'head': head, 'changed_paths': paths, **plan}, indent=2))
    if args.github_output is not None:
        with args.github_output.open('a') as output:
            for name, value in plan.items():
                output.write(f'{name}={json.dumps(value)}\n')


if __name__ == '__main__':
    main()
