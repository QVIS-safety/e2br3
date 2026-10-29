import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_ui_inventory import REPEATABLE_PAGE_ROUTES, build_inventory, normalized


class CanonicalUiInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.registry = Path(__file__).resolve().parents[1]
        cls.frontend = Path(os.environ['E2BR3_FRONTEND_ROOT'])
        cls.document = build_inventory(cls.registry, cls.frontend)

    def test_real_dictionary_has_no_unrepresented_elements(self):
        self.assertEqual([], self.document['coverage']['missingDictionaryElements'])
        self.assertEqual('NOT_RUN', self.document['status'])
        self.assertFalse(self.document['coverage']['executionComplete'])

    def test_known_omissions_come_from_registry_without_census(self):
        paths = {normalized(f['frontendPath']) for f in self.document['fields']}
        for path in ['patientInformation.gpMedicalRecordNumber',
                     'drugs.fdaDevices.deviceBrandName',
                     'drugs.drugReactionAssessments.reactionId',
                     'testResults.testResultQualifier']:
            self.assertIn(path, paths)

    def test_required_conditions_are_sourced_and_routes_are_authority_neutral(self):
        fields = self.document['fields']
        initial = next(f for f in fields if f['code'] == 'C.1.1')
        self.assertEqual('mandatory', initial['required']['criterion'])
        conditional = next(f for f in fields if f['code'] == 'C.1.11.2')
        self.assertEqual('conditional_mandatory', conditional['required']['criterion'])
        self.assertTrue(conditional['required']['conditionText'])
        self.assertTrue(all(f['routes']['editor'].startswith('/{authority}/') for f in fields))

    def test_ci_child_requirements_publish_exact_validator_contracts(self):
        expected = {
            'C.1.6.1.r.1': ('documentsHeldBySender[].documentDescription',
                             'ICH.C.1.6.1.r.1.REQUIRED'),
            'C.1.9.1.r.1': ('otherCaseIdentifiers[].source',
                             'ICH.C.1.9.1.r.1.REQUIRED'),
            'C.1.9.1.r.2': ('otherCaseIdentifiers[].caseIdentifier',
                             'ICH.C.1.9.1.r.2.REQUIRED'),
        }
        for code, (path, rule) in expected.items():
            field = next(item for item in self.document['fields']
                         if item['page'] == 'CI' and item['code'] == code)
            self.assertEqual(field['contractMetadata']['businessValidation'], {
                'status': 'verified', 'ruleCode': rule, 'issuePath': path,
            })
            self.assertIn({'kind': 'requirement_conditional_mandatory'},
                          field['executionBlockers'])

    def test_si_requirements_publish_exact_validator_contracts(self):
        expected = {
            'C.5.4': ('studyInformation.0.studyTypeReaction',
                      'ICH.C.5.4.REQUIRED'),
            'FDA.C.5.5a': ('studyInformation.0.fdaIndNumberOccurred',
                           'FDA.R0024'),
            'FDA.C.5.5b': ('studyInformation.0.fdaPreAndaNumberOccurred',
                           'FDA.R0025'),
            'FDA.C.5.6.r': (
                'studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber',
                'FDA.R0026',
            ),
        }
        for code, (path, rule) in expected.items():
            field = next(item for item in self.document['fields']
                         if item['page'] == 'SI' and item['code'] == code)
            self.assertEqual(field['contractMetadata']['businessValidation'], {
                'status': 'verified', 'ruleCode': rule, 'issuePath': path,
            })
            self.assertIn({'kind': 'requirement_conditional_mandatory'},
                          field['executionBlockers'])

    def test_rp_regional_contracts_publish_required_rule_and_render_dependencies(self):
        email = next(field for field in self.document['fields']
                     if field['identity'] ==
                     'RP|FDA|FDA.C.2.r.2.8|primarySources[].reporterEmail')
        self.assertEqual(email['contractMetadata']['businessValidation'], {
            'status': 'verified',
            'ruleCode': 'FDA.C.2.r.2.8.REQUIRED',
            'issuePath': 'primarySources[].reporterEmail',
        })
        qualification = next(field for field in self.document['fields']
                             if field['identity'] ==
                             'RP|MFDS|C.2.r.4.KR.1|primarySources[].qualificationKr1')
        self.assertEqual(qualification['contractMetadata']['businessValidation'], {
            'status': 'verified',
            'ruleCode': 'MFDS.C.2.r.4.KR.1.REQUIRED',
            'issuePath': 'primarySources[].qualificationKr1',
        })
        self.assertEqual(qualification['contractMetadata']['renderPrerequisites'], [
            {'code': 'C.2.r.local.qualificationNullFlavor',
             'frontendPath': 'primarySources[].qualificationNullFlavor', 'value': None},
            {'code': 'C.2.r.4', 'frontendPath': 'primarySources[].qualification', 'value': '3'},
        ])

    def test_repeatable_routes_match_frontend_route_files(self):
        expected = {
            'RP': (None, '/{authority}/case/{caseId}/detail/RP'),
            'LR': ('/{authority}/case/{caseId}/LR/list', '/{authority}/case/{caseId}/detail/LR/{rowId}'),
            'DH': ('/{authority}/case/{caseId}/DH/list', '/{authority}/case/{caseId}/detail/DH/{rowId}'),
            'AE': ('/{authority}/case/{caseId}/AE/list', '/{authority}/case/{caseId}/detail/AE/{rowId}'),
            'LB': ('/{authority}/case/{caseId}/LB/list', '/{authority}/case/{caseId}/detail/LB/{rowId}'),
            'DG': ('/{authority}/case/{caseId}/DG/list', '/{authority}/case/{caseId}/detail/DG/{rowId}'),
        }
        self.assertEqual(set(expected), set(REPEATABLE_PAGE_ROUTES))
        for page, routes in expected.items():
            field = next(f for f in self.document['fields'] if f['page'] == page)
            self.assertEqual({'list': routes[0], 'editor': routes[1]}, field['routes'])
            detail = self.frontend / 'app/(protected)/[authority]/case/[id]/detail' / page
            editor = detail / ('[rowId]/page.tsx' if '{rowId}' in routes[1] else 'page.tsx')
            self.assertTrue(editor.is_file(), editor)
            if routes[0] is not None:
                listing = self.frontend / 'app/(protected)/[authority]/case/[id]' / page / 'list/page.tsx'
                self.assertTrue(listing.is_file(), listing)

    def test_unknown_hint_cannot_become_a_target_or_disappear(self):
        with tempfile.TemporaryDirectory() as directory:
            hint = Path(directory) / 'hints.json'
            hint.write_text(json.dumps({'fields': [{'identity': 'not-a-real-field',
                                                    'frontendPath': 'drugs.invented'}]}))
            result = build_inventory(self.registry, self.frontend, hint)
        self.assertEqual(['not-a-real-field'], result['coverage']['unmappedExecutionHints'])
        self.assertFalse(result['coverage']['sourceReconciled'])
        self.assertNotIn('drugs.invented', {f['frontendPath'] for f in result['fields']})
        self.assertEqual(len(self.document['fields']), len(result['fields']))

    def test_ci_projection_reconciles_actual_historical_execution_descriptors(self):
        hints = self.registry.parent / 'scripts/case-editor-audit/case-edit-persisted-field-inventory.json'
        result = build_inventory(self.registry, self.frontend, hints)
        coverage = result['coverage']
        self.assertTrue(coverage['sourceReconciled'], {
            key: coverage[key] for key in ('missingDictionaryElements', 'unmappedSourceFields',
                                           'unmappedExecutionHints', 'unmappedContracts')})
        self.assertFalse(coverage['executionComplete'])

    def test_ci_boolean_and_local_dom_bindings_match_rendered_controls(self):
        fields = {field['identity']: field for field in self.document['fields']}
        combination = fields[
            'CI|FDA|FDA.C.1.12|safetyReportIdentification.combinationProductReportIndicator'
        ]
        self.assertEqual(combination['validLiteral'], 'true')
        report_type = fields[
            'CI|FDA|FDA.C.1.local.fdaReportType|case.fdaReportType'
        ]
        self.assertEqual('FDA_REPORT_TYPE',
                         report_type['contractMetadata']['domFieldNumber'])
        mfds_report_type = fields[
            'CI|MFDS|C.1.KR.mfdsReportType|case.mfdsReportType'
        ]
        self.assertEqual('MFDS_REPORT_TYPE',
                         mfds_report_type['contractMetadata']['domFieldNumber'])

    def test_dm_fda_ethnicity_uses_exact_rendered_radix_options(self):
        ethnicity = next(
            field for field in self.document['fields']
            if field['identity'] == 'DM|FDA|FDA.D.12|patientInformation.ethnicityCode'
        )
        self.assertEqual(ethnicity['contractMetadata']['control'], {
            'kind': 'radix_select',
            'options': [
                {'value': 'C17459', 'label': 'C17459 = Hispanic or Latino'},
                {'value': 'C41222', 'label': 'C41222 = Not Hispanic or Latino'},
            ],
        })

    def test_dg_form_autocomplete_inventory_publishes_exact_option_bindings(self):
        fields = {field['identity']: field for field in self.document['fields']}
        additional = fields[
            'DG|FDA|FDA.G.k.10a|drugs[].fdaAdditionalInfoCoded'
        ]
        specialized = fields[
            'DG|FDA|FDA.G.k.10.1|drugs[].fdaSpecializedProductCategory'
        ]
        self.assertEqual('1', additional['validLiteral'])
        self.assertEqual(['1', '2', '3', '4', '5'], [
            option['value'] for option in additional['control']['options']
        ])
        self.assertEqual('C102834', specialized['validLiteral'])
        self.assertNotEqual('1', specialized['validLiteral'])
        self.assertEqual(
            [
                'C102834', 'C102835', 'C102836', 'C102837', 'C102838',
                'C102839', 'C102840', 'C102841', 'C102842',
            ],
            [option['value'] for option in specialized['control']['options']],
        )
        for field in (additional, specialized):
            self.assertEqual('radix_select', field['control']['kind'])
            self.assertEqual(
                field['contractMetadata']['control']['options'],
                field['control']['options'],
            )
            for option in field['control']['options']:
                self.assertTrue(option['label'].startswith(f"{option['value']} = "))
        country = fields[
            'DG|FDA|FDA.G.k.12.r.7.1e|drugs[].fdaDevices[].manufacturerCountry'
        ]
        self.assertEqual('US', country['validLiteral'])
        self.assertEqual({
            'kind': 'async_autocomplete',
            'queryValue': 'US',
            'optionValue': 'US',
            'optionDataValue': 'US',
            'optionLabel': 'US',
            'selectedLabel': 'US',
        }, country['contractMetadata']['control'])
        expected_counts = {
            'DG|MFDS|G.k.9.i.2.r.2.KR.1|drugs[].drugReactionAssessments[].methodOfAssessmentKr1': 2,
            'DG|MFDS|G.k.9.i.2.r.3.KR.1|drugs[].drugReactionAssessments[].resultOfAssessmentKr1': 6,
            'DG|MFDS|G.k.9.i.2.r.3.KR.2|drugs[].drugReactionAssessments[].resultOfAssessmentKr2': 2,
        }
        for identity, option_count in expected_counts.items():
            field = fields[identity]
            self.assertEqual('radix_select', field['control']['kind'])
            self.assertEqual(option_count, len(field['control']['options']))
            self.assertIn(
                field['validLiteral'],
                [option['value'] for option in field['control']['options']],
            )

    def test_missing_execution_evidence_is_not_reported_complete(self):
        for field in self.document['fields']:
            self.assertEqual('NOT_RUN', field['executionEvidence'])
            constraint = field['constraint']
            negative_not_applicable = (isinstance(constraint, dict)
                                      and constraint.get('status') == 'not_applicable'
                                      and isinstance(constraint.get('reason'), str)
                                      and constraint['reason'].strip())
            if field['validLiteral'] is None or (field['invalidLiteral'] is None and not negative_not_applicable):
                self.assertIn(field['identity'], self.document['coverage']['blockedExecutionFields'])

    def test_explicit_contract_prerequisite_is_preserved_as_blocker(self):
        field = next(f for f in self.document['fields']
                     if normalized(f['frontendPath']) == 'drugs.drugReactionAssessments.reactionId')
        blocker = next(b for b in field['executionBlockers']
                       if b['kind'] == 'contract_execution_blocked')
        self.assertEqual(field['contractMetadata']['execution']['reason'], blocker['reason'])
        self.assertIn(field['identity'], self.document['coverage']['blockedExecutionFields'])

    def test_every_blocked_target_explains_its_missing_requirement(self):
        fields = {f['identity']: f for f in self.document['fields']}
        for identity in self.document['coverage']['blockedExecutionFields']:
            self.assertTrue(fields[identity]['executionBlockers'])

    def test_unrendered_companion_preserves_blocked_negative_evidence(self):
        code = 'C.1.local.fulfilExpeditedCriteriaNullFlavor'
        disposition = next(d for d in self.document['dispositions'] if d['code'] == code)
        contract = json.loads((self.registry / 'editor-contracts/ci.json').read_text())
        constraint = next(f['constraint'] for f in contract['fields'] if f['code'] == code)
        self.assertEqual('blocked', constraint['status'])
        self.assertIn({'kind': 'constraint_blocked', 'reason': constraint['reason']},
                      disposition['executionBlockers'])
        self.assertNotIn(code, {f['code'] for f in self.document['fields']})

    def test_product_picker_requires_runtime_reference_without_free_invalid_input(self):
        field = next(f for f in self.document['fields']
                     if normalized(f['frontendPath']) == 'drugs.sourceProductPresaveId')
        self.assertEqual('product_presave_picker', field['control']['kind'])
        self.assertEqual(['InfoList', 'InfoRead'], field['control']['permissionActions'])
        self.assertIsNone(field['validLiteral'])
        self.assertEqual('sourceProductPresaveId', field['payloadPath'])
        constraint = field['contractMetadata']['constraint']
        self.assertEqual('not_applicable', constraint['status'])
        self.assertIsNone(field['invalidLiteral'])
        self.assertNotIn('missing_invalidLiteral', {
            blocker['kind'] for blocker in field['executionBlockers']})
        self.assertNotIn('constraint_blocked', {
            blocker['kind'] for blocker in field['executionBlockers']})
        self.assertIn('contract_execution_blocked', {
            blocker['kind'] for blocker in field['executionBlockers']})

    def test_review_workflow_uses_existing_descriptor_without_scalar_payload(self):
        hints = self.registry.parent / 'scripts/case-editor-audit/case-edit-persisted-field-inventory.json'
        result = build_inventory(self.registry, self.frontend, hints)
        review = [f for f in result['fields'] if f['page'] == 'RE']
        self.assertEqual(3, len(review))
        for field in review:
            descriptor = field['verificationDescriptor']
            self.assertEqual(descriptor['alternateA'], field['validLiteral'])
            self.assertEqual('review_receivers_json_workflow_save', field['writeMechanism'])
            self.assertIsNone(field['payloadPath'])
            self.assertIsNone(field['invalidLiteral'])
            self.assertEqual(field['frontendPath'], field['projectionPath'])
            self.assertEqual(['special_workflow_prerequisite'],
                             [b['kind'] for b in field['executionBlockers']])
            self.assertEqual('NOT_RUN', field['executionEvidence'])

    def test_explicit_local_field_does_not_inherit_official_required_rule(self):
        field = next(f for f in self.document['fields'] if f['code'] == 'C.local.reportYear')
        self.assertEqual('not_applicable', field['required']['criterion'])
        self.assertIn('local_only', field['required']['reason'])
        self.assertEqual('not_applicable', field['constraint']['status'])
        self.assertTrue(field['constraint']['reason'])
        self.assertNotIn({'kind': 'missing_invalidLiteral'}, field['executionBlockers'])
        self.assertEqual('NOT_RUN', field['executionEvidence'])

    def test_source_document_local_field_publishes_plain_audit_item(self):
        field = next(f for f in self.document['fields']
                     if f['code'] == 'C.local.sourceDocumentName')
        self.assertEqual('Source Document Name',
                         field['contractMetadata']['auditItem'])
        self.assertEqual('sourceDocuments[].sourceDocumentName',
                         field['projectionPath'])

    def test_autopsy_cause_text_publishes_exact_production_audit_item(self):
        field = next(f for f in self.document['fields']
                     if f['page'] == 'DM' and f['code'] == 'D.9.4.r.2')
        self.assertEqual('Autopsy Cause of Death Comments (D.9.4.r)',
                         field['contractMetadata']['auditItem'])


if __name__ == '__main__':
    unittest.main()
