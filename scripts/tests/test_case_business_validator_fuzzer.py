#!/usr/bin/env python3

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import case_business_validator_fuzzer as fuzzer


class CaseBusinessValidatorFuzzerTests(unittest.TestCase):
    def test_parser_accepts_explicit_message_header_bindings(self) -> None:
        args = fuzzer.parser().parse_args([
            "--message-sender-identifier", "LOCAL-SENDER",
            "--ich-message-receiver-identifier", "LOCAL-ICH-RECEIVER",
        ])
        self.assertEqual(args.message_sender_identifier, "LOCAL-SENDER")
        self.assertEqual(
            args.ich_message_receiver_identifier, "LOCAL-ICH-RECEIVER",
        )

    def test_catalog_is_seeded_and_ordinals_are_stable(self) -> None:
        first = fuzzer.scenario_catalog(11)
        repeated = fuzzer.scenario_catalog(11)
        another = fuzzer.scenario_catalog(12)
        self.assertEqual(first, repeated)
        self.assertNotEqual(first[1].invalid_value, another[1].invalid_value)
        self.assertEqual([item.ordinal for item in first], list(range(len(first))))
        self.assertEqual(len({item.expected_code for item in first}), len(first))

    def test_inventory_separates_input_contract_rules(self) -> None:
        codes = fuzzer.discover_business_rule_codes()
        self.assertIn("ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE", codes)
        self.assertIn("ICH.G.k.5a.REQUIRED", codes)
        self.assertNotIn("ICH.G.k.2.2.LENGTH.MAX", codes)
        self.assertNotIn("ICH.G.k.1.ALLOWED.VALUE", codes)
        self.assertIn("ICH.G.k.7.r.2a.ALLOWED.VALUE", codes)
        self.assertNotIn("ICH.N.REQUIRED", codes)
        self.assertNotIn("ICH.N.2.r.1.MATCH.C.1.1", codes)

    def test_issue_oracle_requires_complete_evidence(self) -> None:
        report = {"issues": [{
            "code": "RULE",
            "message": "bad relation",
            "path": "drugs.0.value",
            "field_path": "drugs.0.value",
            "section": "G",
            "subsection": "G.k",
        }]}
        self.assertEqual(fuzzer.issue_codes(report), {"RULE"})
        self.assertTrue(fuzzer.issue_complete(report, "RULE"))
        report["issues"][0].pop("path")
        self.assertFalse(fuzzer.issue_complete(report, "RULE"))
        report["issues"][0]["path"] = "drugs.0.value"
        report["issues"][0]["field_path"] = None
        self.assertTrue(fuzzer.issue_complete(report, "RULE"))
        report["issues"][0].pop("field_path")
        self.assertFalse(fuzzer.issue_complete(report, "RULE"))

    def test_scenario_ids_are_unique(self) -> None:
        scenarios = fuzzer.scenario_catalog(1)
        self.assertEqual(len({item.scenario_id for item in scenarios}), len(scenarios))

    def test_ucum_rules_distinguish_direct_parser_from_frequency_release_binding(self) -> None:
        scenarios = fuzzer.scenario_catalog(1)
        strength = next(
            item for item in scenarios
            if item.scenario_id == "g-substance-strength-unit-vocabulary"
        )
        frequency = next(
            item for item in scenarios
            if item.scenario_id == "g-dosage-frequency-unit-vocabulary"
        )
        self.assertFalse(strength.reference_fixture)
        self.assertTrue(frequency.reference_fixture)
        self.assertEqual(
            (
                frequency.invalid_value, frequency.valid_value,
                frequency.fixture_values,
                fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[frequency.scenario_id],
                fuzzer.EXPECTED_SCENARIO_ISSUE_MESSAGES[frequency.scenario_id],
            ),
            (
                "not-a-frequency", "d",
                (("dosageInformation[].numberOfUnits", 1),),
                "drugs.0.dosageInformation.0.frequencyUnit",
                "Dictionary allowed values constraint.",
            ),
        )
        self.assertLessEqual(len(frequency.invalid_value), 50)
        generated_frequency = [
            fuzzer.generated_scenario(frequency, 91, sample)
            for sample in range(3)
        ]
        self.assertTrue(all(
            item.valid_value == "d" and len(item.invalid_value) <= 50
            for item in generated_frequency
        ))
        self.assertEqual(sum(item.reference_fixture for item in scenarios), 42)

    def test_meddra_binding_requires_exact_active_llt_and_proven_absence(self) -> None:
        row = [{
            "active": True,
            "version": "28.1",
            "language": "en",
            "level": "LLT",
            "code": "10000002",
        }]
        releases = [{
            "dictionary": "meddra", "version": "28.1", "language": "en",
            "status": "active", "loaded_rows": 100,
        }]
        self.assertEqual(
            fuzzer.meddra_binding(row, []),
            ("28.1", "10000002", "99999999"),
        )
        with self.assertRaises(ValueError):
            fuzzer.meddra_binding([{**row[0], "level": "PT"}], [])
        with self.assertRaises(ValueError):
            fuzzer.meddra_binding(row, [{"code": "99999999"}])
        with self.assertRaises(ValueError):
            fuzzer.meddra_binding(
                row, [{"code": str(index)} for index in range(100)],
            )
        self.assertEqual(
            fuzzer.meddra_unavailable_version(releases, [], "28.1"), "27.1",
        )
        with self.assertRaises(ValueError):
            fuzzer.meddra_unavailable_version(releases, [{"code": "10000001"}], "28.1")
        self.assertEqual(
            fuzzer.meddra_unavailable_version(releases + [{
                **releases[0], "version": "27.1", "status": "retired",
            }], [], "28.1"),
            "27.1",
        )

    def test_meddra_binding_unblocks_only_code_rules_and_binds_siblings(self) -> None:
        generated = [
            fuzzer.generated_scenario(item, 1, 0)
            for item in fuzzer.scenario_catalog(1)
        ]
        original_fingerprints = {
            item.scenario_id: item.generation_fingerprint for item in generated
        }
        scenarios = fuzzer.bind_meddra_code_scenarios(
            generated, "28.1", "10000002", "99999999",
        )
        bound = [
            item for item in scenarios
            if item.scenario_id in fuzzer.MEDDRA_CODE_SCENARIO_IDS
        ]
        self.assertEqual(len(bound), 12)
        self.assertTrue(all(not item.reference_fixture for item in bound))
        self.assertTrue(all(item.valid_value == "10000002" for item in bound))
        self.assertTrue(all(item.invalid_value == "99999999" for item in bound))
        self.assertTrue(all(
            item.generation_fingerprint == fuzzer.scenario_fingerprint(item)
            for item in bound
        ))
        self.assertTrue(all(
            item.generation_fingerprint != original_fingerprints[item.scenario_id]
            for item in bound
        ))
        context = {
            item.scenario_id: item for item in scenarios
            if item.scenario_id in fuzzer.MEDDRA_CONTEXT_SCENARIO_IDS
        }
        self.assertEqual(set(context), fuzzer.MEDDRA_CONTEXT_SCENARIO_IDS)
        self.assertEqual(
            (context["reactions-collection-required"].invalid_value,
             context["reactions-collection-required"].valid_value),
            (False, True),
        )
        self.assertEqual(
            (context["f-test-meddra-code-required"].invalid_value,
             context["f-test-meddra-code-required"].valid_value),
            (None, "10000002"),
        )
        self.assertEqual(
            (context["reaction-seriousness-null-flavor-ni-only"].invalid_value,
             context["reaction-seriousness-null-flavor-ni-only"].valid_value),
            ("UNK", "NI"),
        )
        self.assertEqual(
            (context["mfds-test-date-null-flavor-vocabulary"].invalid_value,
             context["mfds-test-date-null-flavor-vocabulary"].valid_value),
            ("MSK", "UNK"),
        )
        self.assertTrue(all(not item.reference_fixture for item in context.values()))
        self.assertEqual(
            dict(context["reactions-collection-required"].fixture_values),
            {
                "reactionMeddraVersionLLT": "28.1",
                "reactionMeddraCodeLLT": "10000002",
            },
        )
        self.assertEqual(
            dict(context["f-test-meddra-code-required"].fixture_values),
            {
                "testName": "",
                "testMeddraVersion": "28.1",
                "testMeddraCode": "10000002",
            },
        )
        self.assertEqual(
            dict(context["reaction-seriousness-null-flavor-ni-only"].fixture_values),
            {
                "seriousness.criteriaResultsInDeath": None,
                "reactionMeddraVersionLLT": "28.1",
                "reactionMeddraCodeLLT": "10000002",
            },
        )
        self.assertEqual(
            dict(context["mfds-test-date-null-flavor-vocabulary"].fixture_values),
            {
                "testDate": None,
                "testMeddraVersion": "28.1",
                "testMeddraCode": "10000002",
            },
        )
        past = next(
            item for item in bound
            if item.scenario_id == "d-past-indication-code-vocabulary"
        )
        fixtures = dict(past.fixture_values)
        self.assertEqual(fixtures["indicationMeddraVersion"], "28.1")
        self.assertEqual(fixtures["indicationMeddraCode"], "10000002")
        self.assertEqual(fixtures["reactionMeddraVersion"], "28.1")
        self.assertEqual(fixtures["reactionMeddraCode"], "10000002")
        version_rules = [
            item for item in scenarios
            if item.scenario_id.endswith("-version-vocabulary")
            and item.scenario_id.replace("-version-", "-code-")
            in fuzzer.MEDDRA_CODE_SCENARIO_IDS
        ]
        self.assertEqual(len(version_rules), 12)
        self.assertTrue(all(item.reference_fixture for item in version_rules))

    def test_meddra_version_binding_uses_global_code_and_normalizes_siblings(self) -> None:
        generated = [
            fuzzer.generated_scenario(item, 1, 0)
            for item in fuzzer.scenario_catalog(1)
        ]
        scenarios = fuzzer.bind_meddra_version_scenarios(
            generated, "28.1", "27.1", "10000002",
        )
        bound = [
            item for item in scenarios
            if item.scenario_id in fuzzer.MEDDRA_VERSION_SCENARIO_PATHS
        ]
        self.assertEqual(len(bound), 12)
        self.assertTrue(all(not item.reference_fixture for item in bound))
        self.assertTrue(all(
            item.expected_code == "ICH.MEDDRA.VERSION.UNAVAILABLE"
            and item.invalid_value == "27.1"
            and item.valid_value == "28.1"
            and item.generation_fingerprint == fuzzer.scenario_fingerprint(item)
            for item in bound
        ))
        past = next(
            item for item in bound
            if item.scenario_id == "d-past-indication-version-vocabulary"
        )
        self.assertEqual(dict(past.fixture_values), {
            "indicationMeddraCode": "10000002",
            "indicationMeddraVersion": "28.1",
            "reactionMeddraVersion": "28.1",
            "reactionMeddraCode": "10000002",
        })

    def test_shared_meddra_version_code_requires_exact_unique_path(self) -> None:
        code = "ICH.MEDDRA.VERSION.UNAVAILABLE"
        path = "reactions.0.reactionMeddraVersion"
        issue = {
            "code": code, "message": "not loaded", "path": path,
            "field_path": path, "section": "E", "subsection": "E.i",
        }
        self.assertEqual(fuzzer.complete_issue_count({"issues": [issue]}, code, path), 1)
        self.assertEqual(fuzzer.complete_issue_count(
            {"issues": [{**issue, "field_path": "testResults.0.testMeddraVersion"}]},
            code, path,
        ), 0)
        self.assertEqual(fuzzer.complete_issue_count(
            {"issues": [issue, issue]}, code, path,
        ), 2)

    def test_whodrug_binding_requires_one_matching_active_release_and_product(self) -> None:
        release = [{
            "dictionary": "whodrug", "version": "2026-03", "language": "en",
            "status": "active", "loaded_rows": 688249,
        }]
        product = [{
            "active": True, "version": "2026-03", "language": "en",
            "code": "000754-01-900", "drug_name": "0.225% sodium chloride",
        }]
        self.assertEqual(
            fuzzer.whodrug_binding(release, product),
            ("2026-03", "000754-01-900"),
        )
        with self.assertRaises(ValueError):
            fuzzer.whodrug_binding(release + release, product)
        with self.assertRaises(ValueError):
            fuzzer.whodrug_binding(release, [{**product[0], "version": "2025-09"}])
        with self.assertRaises(ValueError):
            fuzzer.whodrug_binding([{**release[0], "loaded_rows": 0}], product)

    def test_whodrug_binding_unblocks_only_three_version_rules(self) -> None:
        generated = [
            fuzzer.generated_scenario(item, 1, 0)
            for item in fuzzer.scenario_catalog(1)
        ]
        original_fingerprints = {
            item.scenario_id: item.generation_fingerprint for item in generated
        }
        scenarios = fuzzer.bind_whodrug_version_scenarios(
            generated, "2026-03", "000754-01-900",
        )
        bound = [
            item for item in scenarios
            if item.scenario_id in fuzzer.WHODRUG_VERSION_SCENARIOS
        ]
        self.assertEqual(len(bound), 3)
        self.assertTrue(all(not item.reference_fixture for item in bound))
        self.assertTrue(all(item.invalid_value is None for item in bound))
        self.assertTrue(all(item.valid_value == "2026-03" for item in bound))
        self.assertTrue(all(
            dict(item.fixture_values)[fuzzer.WHODRUG_CODE_FIELDS[item.owner]]
            == "000754-01-900" for item in bound
        ))
        self.assertTrue(all(
            item.generation_fingerprint == fuzzer.scenario_fingerprint(item)
            and item.generation_fingerprint != original_fingerprints[item.scenario_id]
            for item in bound
        ))
        payload = {
            "indicationMeddraVersion": "26.0", "indicationMeddraCode": "10000001",
            "reactionMeddraVersion": "26.0", "reactionMeddraCode": "10000001",
            "drugName": "Parent prior drug",
        }
        fuzzer.omit_whodrug_unrelated_meddra(payload, bound[0])
        self.assertEqual(payload, {"drugName": "Parent prior drug"})

    def test_inventory_reports_known_unsupported_rules(self) -> None:
        inventory = fuzzer.discover_business_rule_codes()
        covered = {item.expected_code for item in fuzzer.scenario_catalog(1)}
        dispositions = fuzzer.rule_dispositions()
        test_backed = fuzzer.TEST_BACKED_RULES.keys()
        self.assertEqual((len(inventory), len(covered), len(inventory & covered)), (298, 324, 298))
        self.assertNotIn("ICH.G.k.4.r.10.2a.REQUIRED", inventory)
        self.assertNotIn("ICH.G.k.4.r.10.2a.REQUIRED", covered)
        self.assertNotIn("MFDS.G.k.9.i.2.r.1.REQUIRED", inventory)
        self.assertNotIn("MFDS.G.k.9.i.2.r.1.REQUIRED", covered)
        self.assertNotIn("ICH.C.5.3.REQUIRED", inventory)
        self.assertNotIn("ICH.C.5.3.REQUIRED", covered)
        self.assertEqual(
            inventory - covered - dispositions.keys() - test_backed,
            set(),
        )
        self.assertEqual(dispositions.keys() & covered, set())
        self.assertEqual(test_backed & covered, set())

    def test_current_h1_and_fda_required_intervention_scenarios_remain_catalogued(self) -> None:
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
        }
        self.assertEqual(
            (
                scenarios["singleton-narrative-required"].expected_code,
                scenarios["singleton-narrative-required"].invalid_value,
                scenarios["singleton-narrative-required"].valid_value,
            ),
            ("ICH.H.1.REQUIRED", None, "Business fuzz narrative"),
        )
        self.assertEqual(
            (
                scenarios["fda-required-intervention-required"].expected_code,
                scenarios["fda-required-intervention-required"].invalid_value,
                scenarios["fda-required-intervention-required"].valid_value,
            ),
            ("FDA.E.i.3.2h.REQUIRED", None, True),
        )

    def test_other_case_identifier_format_uses_current_rule_and_iso_control(self) -> None:
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
        }
        self.assertNotIn("c1-other-identifier-profile", scenarios)
        scenario = scenarios["c1-other-identifier-format"]
        self.assertEqual(
            (
                scenario.expected_code, scenario.invalid_value,
                scenario.valid_value, scenario.fixture_values,
                fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[scenario.scenario_id],
                fuzzer.EXPECTED_SCENARIO_ISSUE_MESSAGES[scenario.scenario_id],
            ),
            (
                "ICH.C.1.9.1.r.2.FORMAT", "bad", "KR-ORG-001",
                (("source", "CI source"),),
                "otherCaseIdentifiers.0.caseIdentifier",
                "[C.1.9.1.r.2] Case identifier must use country code-company or regulator name-report number format.",
            ),
        )

    def test_verified_coverage_requires_both_edges_for_every_sample(self) -> None:
        scenarios = [
            fuzzer.generated_scenario(fuzzer.scenario_catalog(1)[0], 1, sample)
            for sample in range(2)
        ]
        events = [
            fuzzer.Event(edge, scenario.scenario_id, scenario.ordinal, scenario.sample_ordinal,
                         scenario.generator_family, scenario.generation_fingerprint, "PASS", 200, {})
            for scenario in scenarios
            for edge in ("invalid_edge", "valid_edge")
        ]
        code = scenarios[0].expected_code
        self.assertEqual(fuzzer.rules_with_both_edges_passed(scenarios, events), {code})
        events[-1].classification = "FAIL"
        self.assertEqual(fuzzer.rules_with_both_edges_passed(scenarios, events), set())

    def test_meddra_version_field_pass_does_not_claim_legacy_rule_coverage(self) -> None:
        scenario = next(
            item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id == "e-reaction-version-vocabulary"
        )
        scenario = fuzzer.bind_meddra_version_scenarios(
            [fuzzer.generated_scenario(scenario, 1, 0)],
            "28.1", "27.1", "10000002",
        )[0]
        events = [
            fuzzer.Event(
                edge, scenario.scenario_id, scenario.ordinal, scenario.sample_ordinal,
                scenario.generator_family, scenario.generation_fingerprint,
                "PASS", 200, {},
            )
            for edge in ("invalid_edge", "valid_edge")
        ]
        self.assertEqual(fuzzer.rules_with_both_edges_passed([scenario], events), set())

    def test_input_contract_rejection_is_not_persisted_both_edge_coverage(self) -> None:
        for scenario_id in (
            "c4-literature-base64-format", "d-lmp-null-flavor-allowed",
            "c1-safety-report-id-required",
            "reaction-seriousness-null-flavor-ni-only",
            "mfds-test-date-null-flavor-vocabulary",
        ):
            scenario = next(
                item for item in fuzzer.scenario_catalog(1)
                if item.scenario_id == scenario_id
            )
            events = [
                fuzzer.Event(
                    "invalid_edge", scenario.scenario_id, scenario.ordinal, 0,
                    scenario.generator_family, scenario.generation_fingerprint,
                    "PASS_INPUT_CONTRACT", 422, {},
                ),
                fuzzer.Event(
                    "valid_edge", scenario.scenario_id, scenario.ordinal, 0,
                    scenario.generator_family, scenario.generation_fingerprint,
                    "PASS", 200, {},
                ),
            ]
            self.assertEqual(
                fuzzer.rules_with_both_edges_passed([scenario], events), set(),
            )

    def test_xml_boundary_events_do_not_count_as_case_rule_coverage(self) -> None:
        scenario = next(
            item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id == "fda-sender-route-pair"
        )
        events = [
            fuzzer.Event(
                edge, scenario.scenario_id, scenario.ordinal, 0,
                scenario.generator_family, scenario.generation_fingerprint,
                "UNVERIFIED_XML_BOUNDARY", 201, {},
            )
            for edge in ("invalid_edge", "valid_edge")
        ]
        self.assertEqual(
            fuzzer.rules_with_both_edges_passed([scenario], events), set(),
        )

    def test_xml_boundary_scenarios_exclude_case_rule_r0109(self) -> None:
        self.assertEqual(len(fuzzer.XML_BOUNDARY_SCENARIO_IDS), 16)
        self.assertIn(
            "fda-sender-route-pair", fuzzer.XML_BOUNDARY_SCENARIO_IDS,
        )
        self.assertNotIn(
            "fda-postmarket-cross-report-forbidden",
            fuzzer.XML_BOUNDARY_SCENARIO_IDS,
        )
        self.assertEqual(
            fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[
                "fda-postmarket-cross-report-forbidden"
            ],
            "studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber",
        )
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
        }
        inventory = fuzzer.discover_business_rule_codes()
        self.assertTrue(fuzzer.XML_BOUNDARY_SCENARIO_IDS <= scenarios.keys())
        self.assertTrue(inventory.isdisjoint({
            scenarios[scenario_id].expected_code
            for scenario_id in fuzzer.XML_BOUNDARY_SCENARIO_IDS
        }))
        self.assertIn(
            scenarios["fda-postmarket-cross-report-forbidden"].expected_code,
            inventory,
        )

    def test_primary_source_audit_alias_matches_storage_field(self) -> None:
        self.assertTrue(fuzzer.audit_key_matches(
            {"primary_source_regulatory": {"old": None, "new": "1"}},
            "primarySourceForRegulatoryPurposes",
        ))

    def test_every_scenario_has_seeded_fallback_free_generation(self) -> None:
        generated = [
            fuzzer.generated_scenario(template, 2026081455, sample)
            for template in fuzzer.scenario_catalog(2026081455)
            for sample in range(3)
        ]
        self.assertEqual(len(generated), 324 * 3)
        self.assertTrue(all(item.generator_family in fuzzer.GENERATOR_FAMILIES for item in generated))
        self.assertEqual(len({item.generation_fingerprint for item in generated}), len(generated))

    def test_scalar_business_scenarios_preserve_invalid_and_valid_edges(self) -> None:
        expected = {
            "d-patient-height-integer": (
                "ICH.D.4.INTEGER", "patientHeight.value", "height_cm",
                "patientInformation.heightCm",
            ),
            "d-parent-height-integer": (
                "ICH.D.10.5.INTEGER", "parentHeight.value", "height_cm",
                "patientInformation.parents.0.heightCm",
            ),
            "d-lmp-null-flavor-allowed": (
                "ICH.D.6.NULLFLAVOR.ALLOWED", "lastMenstrualPeriodDateNullFlavor",
                "last_menstrual_period_date_null_flavor",
                "patientInformation.lastMenstrualPeriodDateNullFlavor",
            ),
            "g-drug-characterization-required": (
                "ICH.G.k.1.REQUIRED", "drugCharacterization",
                "drug_characterization", "drugs.0.drugCharacterization",
            ),
            "g-medicinal-product-required": (
                "ICH.G.k.2.2.REQUIRED", "medicinalProduct",
                "medicinal_product", "drugs.0.medicinalProduct",
            ),
        }
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id in expected
        }
        self.assertEqual(set(scenarios), set(expected))
        for scenario_id, (code, field, projection, path) in expected.items():
            scenario = scenarios[scenario_id]
            self.assertEqual(
                (scenario.expected_code, scenario.field, scenario.projection_field),
                (code, field, projection),
            )
            self.assertEqual(fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[scenario_id], path)

        for scenario_id in ("d-patient-height-integer", "d-parent-height-integer"):
            samples = [
                fuzzer.generated_scenario(scenarios[scenario_id], 91, sample)
                for sample in range(20)
            ]
            self.assertTrue(all(item.invalid_value % 1 != 0 for item in samples))
            self.assertTrue(all(1.1 <= item.invalid_value <= 9.9 for item in samples))
            self.assertTrue(all(len(str(item.invalid_value)) <= 3 for item in samples))
            self.assertTrue(all(item.valid_value % 1 == 0 for item in samples))
        self.assertEqual(scenarios["g-drug-characterization-required"].invalid_value, "")
        self.assertEqual(scenarios["g-medicinal-product-required"].invalid_value, "")
        self.assertEqual(
            (scenarios["d-lmp-null-flavor-allowed"].invalid_value,
             scenarios["d-lmp-null-flavor-allowed"].valid_value),
            ("NASK", "MSK"),
        )

    def test_supplemental_retained_rules_keep_business_edges_and_exact_oracles(self) -> None:
        expected = {
            "d-concomitant-therapy-allowed-value": (
                "ICH.D.7.3.ALLOWED.VALUE", "concomitantTherapies",
                "concomitant_therapy", False, True,
                "patientInformation.concomitantTherapy",
                "Dictionary allowed values constraint.",
            ),
            "d-family-history-allowed-value": (
                "ICH.D.7.1.r.6.ALLOWED.VALUE", "familyHistory",
                "family_history", False, True,
                "patientInformation.medicalHistory.0.familyHistory",
                "Dictionary allowed values constraint.",
            ),
            "d-parent-age-unit-allowed-value": (
                "ICH.D.10.2.2b.ALLOWED.VALUE", "parentAge.unit",
                "parent_age_unit", "mo", "a",
                "patientInformation.parents.0.parentAgeUnit",
                "[D.10.2.2b] Parent age unit must be years (a) or decades (10.a).",
            ),
            "g-investigational-product-allowed-value": (
                "ICH.G.k.2.5.ALLOWED.VALUE", "investigationalProductBlinded",
                "investigational_product_blinded", False, True,
                "drugs.0.investigationalProductBlinded",
                "Dictionary allowed values constraint.",
            ),
        }
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id in fuzzer.SUPPLEMENTAL_RETAINED_SCENARIO_IDS
        }
        self.assertEqual(set(scenarios), set(expected))
        for scenario_id, values in expected.items():
            scenario = scenarios[scenario_id]
            self.assertEqual(
                (
                    scenario.expected_code, scenario.field,
                    scenario.projection_field, scenario.invalid_value,
                    scenario.valid_value,
                    fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[scenario_id],
                    fuzzer.EXPECTED_SCENARIO_ISSUE_MESSAGES[scenario_id],
                ),
                values,
            )
        family = scenarios["d-family-history-allowed-value"]
        generated_family = fuzzer.generated_scenario(family, 91, 0)
        bound_family = fuzzer.bind_meddra_code_scenarios(
            [generated_family], "28.1", "10000002", "99999999",
        )[0]
        self.assertFalse(bound_family.invalid_value)
        self.assertTrue(bound_family.valid_value)
        self.assertEqual(
            dict(bound_family.fixture_values),
            {"meddraVersion": "28.1", "meddraCode": "10000002"},
        )
        self.assertFalse(bound_family.reference_fixture)
        blinded = scenarios["g-investigational-product-allowed-value"]
        self.assertEqual(dict(blinded.ci_values), {"reportType": "2"})
        self.assertEqual(dict(blinded.study_values), {"studyTypeReaction": "1"})
        units = [
            fuzzer.generated_scenario(
                scenarios["d-parent-age-unit-allowed-value"], 91, sample,
            ).valid_value
            for sample in range(3)
        ]
        self.assertEqual(units, ["a", "10.a", "a"])

        report = {"issues": [{
            "code": expected["d-concomitant-therapy-allowed-value"][0],
            "message": expected["d-concomitant-therapy-allowed-value"][6],
            "path": expected["d-concomitant-therapy-allowed-value"][5],
            "field_path": expected["d-concomitant-therapy-allowed-value"][5],
            "section": "patient-information",
            "subsection": "D.7",
        }]}
        self.assertEqual(
            fuzzer.complete_issue_count(
                report, report["issues"][0]["code"],
                report["issues"][0]["field_path"],
                report["issues"][0]["message"],
            ),
            1,
        )
        self.assertEqual(
            fuzzer.complete_issue_count(
                report, report["issues"][0]["code"],
                report["issues"][0]["field_path"], "wrong message",
            ),
            0,
        )

    def test_collection_and_test_name_scenarios_keep_distinct_edges(self) -> None:
        expected = {
            "primary-sources-collection-required": (
                "ICH.C.2.r.REQUIRED", False, True, "primarySources",
            ),
            "drugs-collection-required": (
                "ICH.G.k.REQUIRED", False, True, "drugs",
            ),
            "f-test-name-group-required": (
                "ICH.F.r.2.REQUIRED", "", "ALT", "testResults.0.testName",
            ),
            "f-test-name-text-required": (
                "ICH.F.r.2.1.REQUIRED", "", "ALT", "testResults.0.testName",
            ),
            "reactions-collection-required": (
                "ICH.E.i.REQUIRED", False, True, "reactions",
            ),
            "f-test-meddra-code-required": (
                "ICH.F.r.2.2b.REQUIRED", None, "BOUND-MEDDRA-CODE",
                "testResults.0.testMeddraCode",
            ),
            "reaction-seriousness-null-flavor-ni-only": (
                "ICH.E.i.3.2.NI.ONLY", "UNK", "NI",
                "reactions.0.seriousnessCriteria",
            ),
            "mfds-test-date-null-flavor-vocabulary": (
                "MFDS.F.r.1.NULLFLAVOR.VOCABULARY", "MSK", "UNK",
                "testResults.0.testDateNullFlavor",
            ),
            "fda-device-collection-required": (
                "FDA.G.k.12.COLLECTION.REQUIRED", False, True,
                "drugs.0.fdaDevices",
            ),
            "fda-assessment-collection-required": (
                "FDA.G.k.9.REQUIRED", False, True,
                "drugs.0.drugReactionAssessments",
            ),
        }
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id in expected
        }
        self.assertEqual(set(scenarios), set(expected))
        for scenario_id, (code, invalid, valid, path) in expected.items():
            scenario = scenarios[scenario_id]
            self.assertEqual(
                (scenario.expected_code, scenario.invalid_value, scenario.valid_value),
                (code, invalid, valid),
            )
            self.assertEqual(
                fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[scenario_id], path,
            )
        for scenario_id in (
            "f-test-name-group-required", "f-test-name-text-required",
        ):
            self.assertEqual(
                dict(scenarios[scenario_id].fixture_values),
                {"testMeddraVersion": None, "testMeddraCode": None},
            )
        for scenario_id in (
            "primary-sources-collection-required", "drugs-collection-required",
        ):
            samples = [
                fuzzer.generated_scenario(scenarios[scenario_id], 91, sample)
                for sample in range(3)
            ]
            self.assertTrue(all(item.invalid_value is False for item in samples))
            self.assertTrue(all(item.valid_value is True for item in samples))

    def test_c1_singletons_distinguish_input_rejection_from_persisted_clear(self) -> None:
        scenarios = {
            item.scenario_id: item for item in fuzzer.scenario_catalog(91)
            if item.scenario_id in {
                "c1-safety-report-id-required", "c1-transmission-date-required",
            }
        }
        self.assertEqual(set(scenarios), {
            "c1-safety-report-id-required", "c1-transmission-date-required",
        })
        self.assertEqual(
            (
                scenarios["c1-safety-report-id-required"].expected_code,
                scenarios["c1-safety-report-id-required"].invalid_value,
                fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[
                    "c1-safety-report-id-required"
                ],
            ),
            (
                "ICH.C.1.1.REQUIRED", None,
                "safetyReportIdentification.safetyReportId",
            ),
        )
        self.assertEqual(
            (
                scenarios["c1-transmission-date-required"].expected_code,
                scenarios["c1-transmission-date-required"].invalid_value,
                fuzzer.EXPECTED_SCENARIO_ISSUE_PATHS[
                    "c1-transmission-date-required"
                ],
            ),
            (
                "ICH.C.1.2.REQUIRED", None,
                "safetyReportIdentification.transmissionDate",
            ),
        )

    def test_generation_is_reproducible_and_sampled(self) -> None:
        template = next(
            item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id == "c1-transmission-date-future"
        )
        first = fuzzer.generated_scenario(template, 99, 0)
        self.assertEqual(first, fuzzer.generated_scenario(template, 99, 0))
        samples = [fuzzer.generated_scenario(template, 99, sample) for sample in range(3)]
        self.assertEqual(len({item.invalid_value for item in samples}), 3)
        self.assertTrue(all(int(item.invalid_value[:4]) >= 2030 for item in samples))
        self.assertTrue(all(int(item.valid_value[:4]) <= 2024 for item in samples))

    def test_unknown_generator_family_fails_instead_of_falling_back(self) -> None:
        template = fuzzer.Scenario(
            0, "unknown", "ich", "CI", "owner", "field", "field", "UNKNOWN", {}, {},
        )
        with self.assertRaises(ValueError):
            fuzzer.generated_scenario(template, 1, 0)
        unsupported_string = fuzzer.Scenario(
            0, "unknown-string", "ich", "CI", "owner", "opaque", "opaque",
            "UNKNOWN.REQUIRED", None, "value",
        )
        with self.assertRaises(ValueError):
            fuzzer.generated_scenario(unsupported_string, 1, 0)

    def test_lexical_candidates_are_real_seeded_values(self) -> None:
        template = next(
            item for item in fuzzer.scenario_catalog(1)
            if item.scenario_id == "c1-document-base64-required"
        )
        samples = [fuzzer.generated_scenario(template, 77, sample) for sample in range(3)]
        self.assertEqual(len({item.invalid_value for item in samples}), 3)
        self.assertEqual(len({item.valid_value for item in samples}), 3)
        self.assertTrue(all(item.invalid_value.startswith("%%%") for item in samples))


if __name__ == "__main__":
    unittest.main()
