#!/usr/bin/env python3
"""Read the complete event diff, including both sides of renames."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess


def changed_paths(base: str, head: str, pull_request: bool, root: Path) -> list[str]:
    if not base or not head or base.startswith('-') or head.startswith('-'):
        raise ValueError('explicit base and head revisions are required')

    def git(*args: str) -> bytes:
        return subprocess.check_output(['git', *args], cwd=root)

    if base == '0' * 40:
        if pull_request:
            raise ValueError('pull request requires a real base commit')
        raw = git('ls-tree', '-r', '--name-only', '-z', head)
    else:
        if pull_request:
            base = git('merge-base', base, head).decode().strip()
        raw = git('diff', '--no-ext-diff', '--no-renames', '--name-only', '-z', base, head, '--')
    return [os.fsdecode(path) for path in raw.split(b'\0') if path]


def event_revisions() -> tuple[str, str, str]:
    event_name = os.environ['GITHUB_EVENT_NAME']
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if event_name == 'pull_request':
        return event_name, event['pull_request']['base']['sha'], event['pull_request']['head']['sha']
    if event_name == 'push':
        return event_name, event['before'], event['after']
    if event_name == 'workflow_dispatch':
        return event_name, '0' * 40, os.environ['GITHUB_SHA']
    raise ValueError(f'unsupported CI event: {event_name}')
