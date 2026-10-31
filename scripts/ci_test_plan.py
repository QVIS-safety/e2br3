#!/usr/bin/env python3
"""Select complete integration targets for test-only PRs; pushes always run all tests."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess


TARGETS = {
    'web-server': ('api', 'authz', 'smoke', 'isolation_guard', 'reaction_term_highlight_migration'),
    'lib-core': ('architecture_validation_boundary', 'authorization_contract_snapshot',
                 'authorization_kernel', 'null_flavor_storage_contract', 'rbac_grant_profiles',
                 'regulatory_authority', 'section_presave', 'terminology'),
    'xml': ('import', 'validation_api', 'xml'),
}
ROOTS = {
    'web-server': 'crates/services/web-server/tests/',
    'lib-core': 'crates/libs/lib-core/tests/',
    'xml': 'crates/libs/xml/tests/',
}
FAMILIES = {
    'web-server': {'api/terminology_web.rs': 'smoke', 'api/': 'api', 'authz/': 'authz', 'xml/': 'smoke'},
    'lib-core': {'ui/authorization/': 'authorization_kernel',
                 'snapshots/authorization_catalog.sha256': 'authorization_contract_snapshot',
                 'terminology/': 'terminology'},
    'xml': {'import/': 'import', 'xml/': 'xml'},
}


def target_for(path: str):
    for package, root in ROOTS.items():
        if not path.startswith(root):
            continue
        relative = path[len(root):]
        for family, target in FAMILIES[package].items():
            if relative == family or (family.endswith('/') and relative.startswith(family)):
                return f'{package}:{target}'
        for target in TARGETS[package]:
            if relative == f'{target}.rs':
                return f'{package}:{target}'
    return None


def select_tests(event: str, paths: list[str], root: Path = Path('.')) -> dict:
    full = {'mode': 'full', 'targets': [], 'backend_contracts': True}
    if event == 'push':
        return full
    if event != 'pull_request':
        raise ValueError(f'unsupported CI event: {event}')
    if not paths:
        return full
    selected = set()
    for path in paths:
        if path in ('README.md', 'AGENTS.md') or (path.startswith('docs/') and path.endswith('.md')):
            continue
        target = target_for(path)
        if target is None:
            return full
        selected.add(target)
    for target in selected:
        package, name = target.split(':')
        if not (root / ROOTS[package] / f'{name}.rs').is_file():
            return full
    return {'mode': 'targeted' if selected else 'none', 'targets': sorted(selected),
            'backend_contracts': 'web-server:api' in selected}


def changed_paths(base: str, head: str) -> list[str]:
    if not base or not head or base.startswith('-') or head.startswith('-'):
        raise ValueError('explicit base and head revisions are required')
    # Include both sides of a rename so moving a shared file cannot narrow the run.
    output = subprocess.check_output([
        'git', 'diff', '--name-only', '--no-renames', '-z', f'{base}...{head}', '--',
    ])
    return [path.decode('utf-8') for path in output.split(b'\0') if path]


def target_commands(targets: list[str]) -> list[list[str]]:
    allowed = {f'{package}:{target}' for package, names in TARGETS.items() for target in names}
    if not isinstance(targets, list) or not targets or any(not isinstance(t, str) for t in targets):
        raise ValueError('selected targets must be a nonempty list of target IDs')
    if len(targets) != len(set(targets)) or not set(targets) <= allowed:
        raise ValueError('unknown or duplicate integration targets')
    commands = []
    for package, names in TARGETS.items():
        selected = [name for name in names if f'{package}:{name}' in targets]
        if selected:
            args = ['scripts/test-isolated-db.sh', '-p', package, '-j1']
            for name in selected:
                args.extend(['--test', name])
            commands.append(args + ['--', '--test-threads=1', '--color', 'never'])
    return commands


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest='command', required=True)
    plan_parser = subparsers.add_parser('plan')
    plan_parser.add_argument('--event', choices=('push', 'pull_request'), required=True)
    plan_parser.add_argument('--base')
    plan_parser.add_argument('--head', required=True)
    plan_parser.add_argument('--github-output', type=Path)
    run_parser = subparsers.add_parser('run')
    run_parser.add_argument('--targets', required=True)
    args = parser.parse_args()
    if args.command == 'run':
        for command in target_commands(json.loads(args.targets)):
            subprocess.run(command, check=True)
        return
    paths = changed_paths(args.base, args.head) if args.event == 'pull_request' else []
    plan = select_tests(args.event, paths)
    print(json.dumps({'changed_paths': paths, **plan}))
    if args.github_output is not None:
        with args.github_output.open('a') as output:
            for name, value in plan.items():
                output.write(f'{name}={value if isinstance(value, str) else json.dumps(value)}\n')


if __name__ == '__main__':
    main()
