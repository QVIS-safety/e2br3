#!/usr/bin/env python3
"""Run changed crates and their consumers; test-only edits stay in their owning targets."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess

from ci_test_scope import changed_paths, event_revisions

# Nested modules whose directory name differs from their Cargo integration target.
FAMILIES = {
    'lib-core': {'ui/authorization/': 'authorization_kernel',
                 'snapshots/authorization_catalog.sha256': 'authorization_contract_snapshot',
                 'presave/': 'section_presave'},
}

CONTRACT_TARGETS = {'api_cases', 'api_editor', 'api_import'}


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
    for path in paths:
        if Path(path).name in ('README.md', 'AGENTS.md', 'subagent.md') or (path.startswith('docs/') and path.endswith('.md')):
            continue
        owner = next((name for name, data in workspace.items() if path.startswith(data['root'])), None)
        if owner is not None:
            relative = path[len(workspace[owner]['root']):]
            if relative.startswith('tests/'):
                target = target_for(owner, relative, workspace)
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
    backend_contracts = 'web-server' in selected and (
        not selected['web-server'] or bool(CONTRACT_TARGETS.intersection(selected['web-server'])))
    return {
        'selections': selected,
        'validator': 'validator' in selected,
        'backend': any(name != 'validator' for name in selected),
        'registry': backend_contracts,
        'backend_contracts': backend_contracts,
        'rust': bool(selected),
        'xml_assets': bool({'web-server', 'xml'} & selected.keys()),
        'full_reasons': full_reasons,
    }


def commands_for(selections: dict, workspace: dict, kind: str = 'tests') -> list[list[str]]:
    if not isinstance(selections, dict) or not selections:
        raise ValueError('selections must be a nonempty package-to-targets object')
    for package, targets in selections.items():
        if package not in workspace or not isinstance(targets, list) or any(not isinstance(t, str) for t in targets):
            raise ValueError('unknown package or invalid target list')
        if len(set(targets)) != len(targets) or not set(targets) <= workspace[package]['targets'].keys():
            raise ValueError('unknown or duplicate integration targets')
    package_args = [arg for package in sorted(selections) for arg in ('-p', package)]
    if kind == 'fmt':
        return [['cargo', 'fmt', *package_args, '--check']]
    if kind == 'clippy':
        return [['cargo', 'clippy', '--locked', '--all-targets', '--all-features', *package_args]]
    if kind != 'tests':
        raise ValueError(f'unknown check: {kind}')
    commands = []
    for package, targets in sorted(selections.items()):
        if package == 'validator':  # Dedicated job also verifies all rule bindings.
            continue
        args = ['scripts/test-isolated-db.sh', '--locked', '-p', package, '-j1']
        for target in targets:
            args.extend(['--test', target])
        commands.append(args + ['--', '--test-threads=1', '--color', 'never'])
    return commands


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
    run_parser.add_argument('--kind', choices=('tests', 'fmt', 'clippy'), default='tests')
    args = parser.parse_args()
    root = Path.cwd()
    workspace = load_workspace(root)
    if args.command == 'run':
        for command in commands_for(json.loads(args.selections), workspace, args.kind):
            print('Running:', ' '.join(command), flush=True)
            subprocess.run(command, check=True)
        return
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
