#!/usr/bin/env python3
"""Export Jev execution targets from the canonical registry, never a fixed census."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
from pathlib import Path


ROOT_PAGES = {
    'case': 'CI', 'safetyReportIdentification': 'CI', 'caseSummaryInformation': 'CI',
    'documentsHeldBySender': 'CI', 'sourceDocuments': 'CI', 'linkedReports': 'CI',
    'otherCaseIdentifiers': 'CI', 'primarySources': 'RP', 'senderInformation': 'SD',
    'receiverInformation': 'SD', 'studyInformation': 'SI', 'patientInformation': 'DM',
    'reactions': 'AE', 'testResults': 'LB', 'drugs': 'DG', 'literatureReferences': 'LR',
    'narrative': 'NR', 'reviewReceivers': 'RE', 'messageHeader': 'N',
}
REPEATABLE_PAGES = {'RP', 'LR', 'DH', 'AE', 'LB', 'DG'}
REPEATABLE_PAGE_ROUTES = {
    'RP': {'list': None, 'editorRow': False},
    'LR': {'list': True, 'editorRow': True},
    'DH': {'list': True, 'editorRow': True},
    'AE': {'list': True, 'editorRow': True},
    'LB': {'list': True, 'editorRow': True},
    'DG': {'list': True, 'editorRow': True},
}


def normalized(path: str) -> str:
    return re.sub(r'\.\d+(?=\.|$)', '', path.replace('[]', ''))


def validate_execution_ledger(ledger: dict, inventory: dict) -> None:
    if ledger.get('schemaVersion') != 1:
        raise ValueError('execution ledger schemaVersion must be 1')
    if ledger.get('status') != 'COMPLETE' or ledger.get('wholeCanonicalComplete') is not True:
        raise ValueError('execution ledger must record complete canonical coverage')

    fields = inventory.get('fields')
    records = ledger.get('records')
    proofs = ledger.get('proofs')
    if not isinstance(fields, list) or not isinstance(records, list) or not isinstance(proofs, list):
        raise ValueError('execution ledger and canonical inventory rows must be arrays')

    canonical = {field.get('identity'): field for field in fields}
    recorded = {record.get('identity'): record for record in records}
    proven = {proof.get('identity'): proof for proof in proofs}
    if None in canonical or len(canonical) != len(fields):
        raise ValueError('canonical inventory identities must be present and unique')
    if None in recorded or len(recorded) != len(records):
        raise ValueError('execution ledger record identities must be present and unique')
    if None in proven or len(proven) != len(proofs):
        raise ValueError('execution ledger proof identities must be present and unique')
    if recorded.keys() != canonical.keys() or proven.keys() != canonical.keys():
        raise ValueError('execution ledger identities do not match the canonical inventory')

    static_blockers = 0
    sha256 = re.compile(r'^[0-9a-f]{64}$')
    for identity, field in canonical.items():
        blocked = bool(field.get('executionBlockers'))
        static_blockers += blocked
        record = recorded[identity]
        proof = proven[identity]
        if (record.get('state') != 'VERIFIED'
                or record.get('proofRecorded') is not True
                or record.get('staticBlocker') is not blocked):
            raise ValueError(f'{identity}: execution record is incomplete or stale')
        result = proof.get('result')
        if (proof.get('outcome') not in {'VERIFIED', 'VERIFIED_ROUNDTRIP'}
                or not isinstance(result, dict)
                or not isinstance(result.get('path'), str)
                or not sha256.fullmatch(str(result.get('sha256', '')))
                or not sha256.fullmatch(str(proof.get('proofContractSha256', '')))):
            raise ValueError(f'{identity}: execution proof is incomplete or unpinned')

    expected_counts = {
        'canonicalFieldCount': len(canonical),
        'staticBlockerCount': static_blockers,
        'verifiedProofCount': len(canonical),
        'verifiedStaticBlockerCount': static_blockers,
        'evidenceAdjustedUnprovedCount': 0,
        'connectedFieldCount': len(canonical),
        'unprovedFieldCount': 0,
        'readyEvidenceRequiredCount': 0,
        'pendingStaticBlockerCount': 0,
    }
    if ledger.get('counts') != expected_counts:
        raise ValueError('execution ledger counts do not match the canonical inventory')


def build_inventory(registry_root: Path, frontend_root: Path,
                    execution_hints_path: Path | None = None) -> dict:
    registry_root = Path(registry_root).resolve()
    frontend_root = Path(frontend_root).resolve()
    inputs = {}

    def read(path):
        raw = path.read_bytes()
        inputs[str(path)] = hashlib.sha256(raw).hexdigest()
        return json.loads(raw)

    rows = [row for name in read(registry_root / 'index.json')['sections']
            for row in read(registry_root / name)]
    dictionaries = {}
    for path in sorted((registry_root / 'dictionary').glob('*.json')):
        document = read(path)
        for entry in document['entries']:
            code = entry['code']
            if code in dictionaries:
                raise ValueError(f'duplicate dictionary code: {code}')
            dictionaries[code] = dict(entry, authority=document['authority'])
    contracts = {}
    for path in sorted((registry_root / 'editor-contracts').glob('*.json')):
        if path.name == 'schema.json':
            continue
        contract = read(path)
        for field in contract['fields']:
            key = normalized(field['frontendPath'])
            if key in contracts:
                raise ValueError(f'duplicate contract path: {key}')
            contracts[key] = dict(field, page=contract['pageId'])
    hints = {}
    if execution_hints_path is not None:
        for field in read(Path(execution_hints_path))['fields']:
            key = normalized(field['frontendPath'])
            if key in hints:
                raise ValueError(f'duplicate execution hint: {key}')
            hints[key] = field
    extractor = registry_root / 'tools/extract_frontend_fields.mjs'
    inputs[str(extractor)] = hashlib.sha256(extractor.read_bytes()).hexdigest()
    completed = subprocess.run(['node', str(extractor), str(frontend_root)],
                               capture_output=True, text=True, check=True)
    extracted = json.loads(completed.stdout)
    source = {normalized(item['key']): item for item in extracted}
    for item in extracted:
        source_path = (frontend_root.parent / item['file']).resolve()
        if frontend_root not in source_path.parents:
            raise ValueError(f'extractor source outside frontend root: {source_path}')
        inputs[str(source_path)] = hashlib.sha256(source_path.read_bytes()).hexdigest()
    fields, dispositions, represented = [], [], set()
    registered_codes = {r['e2br3_code'] for r in rows}
    missing_codes = sorted(code for code, entry in dictionaries.items()
                           if entry['kind'] == 'element' and code not in registered_codes)
    for row in rows:
        front = row['frontend']
        code = row['e2br3_code']
        if front['status'] != 'mapped':
            disposition_paths = []
            if isinstance(front.get('section'), str) and isinstance(front.get('field'), str):
                disposition_paths = [normalized(front['section'] + '.' + leaf.strip())
                                     for leaf in front['field'].split('/')]
                represented.update(disposition_paths)
            dispositions.append({'code': code, 'authority': row['authority'],
                                 'status': row['status'], 'frontend': front,
                                 'backend': row['backend'], 'frontendPaths': disposition_paths,
                                 'executionBlockers': [
                                     {'kind': 'constraint_blocked', 'reason': contracts[path]['constraint']['reason']}
                                     for path in disposition_paths if path in contracts
                                     and contracts[path].get('constraint', {}).get('status') == 'blocked'],
                                 'executionEvidence': 'NOT_RUN'})
            continue
        for leaf in front['field'].split('/'):
            key = normalized(front['section'] + '.' + leaf.strip())
            if key in represented:
                raise ValueError(f'duplicate registry UI path: {key}')
            represented.add(key)
            contract = contracts.get(key)
            hint = hints.get(key)
            root = key.split('.')[0]
            if root not in ROOT_PAGES:
                raise ValueError(f'unknown UI root: {key}')
            page = row.get('editor_page')
            if page is None:
                page = ROOT_PAGES[root]
                if key.startswith('patientInformation.pastDrugHistory.'):
                    page = 'DH'
            if page == 'N':
                dispositions.append({'code': code, 'authority': row['authority'],
                                     'status': 'non_case_editor_header', 'frontendPath': key,
                                     'evidence': front})
                continue
            field = copy.deepcopy(hint) if hint is not None else {}
            path = contract['frontendPath'] if contract is not None else (
                hint['frontendPath'] if hint is not None else key)
            authority = row['authority']
            definition = dictionaries.get(code)
            companion = leaf.strip().endswith('NullFlavor')
            conformance = definition.get('conformance') if definition is not None else None
            criterion = 'optional' if companion else conformance
            if criterion is None and row.get('local_only') is True:
                criterion = 'not_applicable'
            if criterion is None:
                criterion = 'unresolved'
            field.update({
                'identity': f'{page}|{authority}|{code}|{path}', 'page': page,
                'authority': authority, 'code': code, 'frontendPath': path,
                'registryId': row['id'], 'registryStatus': row['status'],
                'owner': contract['patch']['owner'] if contract is not None else (
                    hint['owner'] if hint is not None else root),
                'payloadPath': contract['payloadPath'] if contract is not None else None,
                'projectionPath': contract['projectionPath'] if contract is not None else None,
                'routes': {
                    'list': (f'/{{authority}}/case/{{caseId}}/{page}/list'
                             if REPEATABLE_PAGE_ROUTES.get(page, {}).get('list') else None),
                    'editor': (f'/{{authority}}/case/{{caseId}}/detail/{page}'
                               + ('/{rowId}' if REPEATABLE_PAGE_ROUTES.get(page, {}).get('editorRow') else '')),
                },
                'uiClassification': 'mounted_editable' if key in source else 'dom_reconciliation_required',
                'control': {'kind': 'dom_inspection_required',
                            **(contract.get('control', {}) if contract is not None else {}),
                            'source': source.get(key),
                            'evidenceLimit': 'Source binding is not proof of rendered/editable UI.'},
                'prerequisites': ['editable dedicated Case', 'explicit existing outer row ID'] if page in REPEATABLE_PAGES else ['editable dedicated Case'],
                'validLiteral': contract.get('roundTripValue') if contract is not None else None,
                'invalidLiteral': contract.get('constraint', {}).get('invalidValue') if contract is not None else None,
                'constraint': contract.get('constraint') if contract is not None else None,
                'required': {'criterion': criterion, 'conformance': conformance,
                             'conditionText': definition.get('condition_text') if definition is not None else None,
                             'profiles': definition.get('profiles') if definition is not None else None,
                             'source': 'registry/dictionary' if definition is not None else 'local registry mapping; no official requirement',
                             'reason': 'Registry explicitly marks this field local_only; official conformance does not apply. Local input constraints remain separate.' if criterion == 'not_applicable' else None,
                             'companion': companion},
                'definition': definition,
                'executionEvidence': 'NOT_RUN',
                'contractMetadata': contract,
            })
            blockers = []
            descriptor = field.get('verificationDescriptor')
            special_review = (page == 'RE' and field.get('category') == 'write_special_json'
                              and isinstance(descriptor, dict)
                              and descriptor.get('writeMechanism') == 'review_receivers_json_workflow_save')
            if special_review:
                if not all(isinstance(descriptor.get(k), str) and descriptor[k]
                           for k in ('alternateA', 'alternateB')):
                    raise ValueError(f'{code}: review workflow descriptor lacks explicit alternates')
                prerequisites = descriptor.get('prerequisites')
                if not isinstance(prerequisites, list) or not prerequisites:
                    raise ValueError(f'{code}: review workflow descriptor lacks prerequisites')
                field.update(writeMechanism=descriptor['writeMechanism'],
                             projectionPath=path, validLiteral=descriptor['alternateA'])
                field['prerequisites'].extend(prerequisites)
                blockers.append({'kind': 'special_workflow_prerequisite',
                                 'reason': 'Requires an explicitly bound existing Review row, whole-collection persistence and descriptor-specific clear checks.',
                                 'prerequisites': prerequisites})
            for name in (() if special_review else ('validLiteral', 'invalidLiteral', 'payloadPath')):
                if name == 'invalidLiteral' and isinstance(field['constraint'], dict):
                    constraint = field['constraint']
                    if constraint.get('status') == 'not_applicable' and isinstance(constraint.get('reason'), str) and constraint['reason'].strip():
                        continue
                if field[name] is None:
                    blockers.append({'kind': 'missing_' + name})
            if criterion in {'unresolved', 'conditional_mandatory'}:
                blockers.append({'kind': 'requirement_' + criterion})
            if field['uiClassification'] != 'mounted_editable':
                blockers.append({'kind': 'dom_reconciliation_required'})
            if contract is not None:
                constraint = contract.get('constraint')
                if constraint is not None and constraint['status'] == 'blocked':
                    blockers.append({'kind': 'constraint_blocked', 'reason': constraint['reason']})
                execution = contract.get('execution')
                if execution is not None and execution['status'] == 'blocked':
                    blockers.append({'kind': 'contract_execution_blocked',
                                     'reason': execution['reason']})
            field['executionBlockers'] = blockers
            fields.append(field)
    source_gaps = [item for item in extracted if normalized(item['key']) not in represented]
    hint_gaps = [field['identity'] for key, field in hints.items() if key not in represented]
    contract_gaps = [field for key, field in contracts.items() if key not in represented]
    blocked = [f['identity'] for f in fields if f['executionBlockers']]
    return {'schemaVersion': 2, 'source': 'canonical_registry', 'status': 'NOT_RUN',
            'fields': fields, 'dispositions': dispositions, 'sourceHashes': inputs,
            'coverage': {'fieldCount': len(fields), 'missingDictionaryElements': missing_codes,
                         'unmappedSourceFields': source_gaps, 'unmappedExecutionHints': hint_gaps,
                         'unmappedContracts': contract_gaps, 'blockedExecutionFields': blocked,
                         'sourceReconciled': not (missing_codes or source_gaps or hint_gaps or contract_gaps),
                         'executionComplete': False}}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--registry-root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--frontend-root', type=Path, required=True)
    parser.add_argument('--execution-hints', type=Path)
    parser.add_argument('--execution-ledger', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--check', action='store_true', help='fail on unmapped source/contract/hint paths')
    args = parser.parse_args()
    result = build_inventory(args.registry_root, args.frontend_root, args.execution_hints)
    if args.execution_ledger is not None:
        validate_execution_ledger(json.loads(args.execution_ledger.read_text()), result)
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
    if args.out is not None:
        args.out.write_text(encoded)
    else:
        print(encoded, end='')
    return 1 if args.check and not result['coverage']['sourceReconciled'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
