#!/usr/bin/env python3

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import case_business_validator_fuzzer as fuzzer


class CaseBusinessValidatorFuzzerTests(unittest.TestCase):
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

    def test_inventory_reports_known_unsupported_rules(self) -> None:
        inventory = fuzzer.discover_business_rule_codes()
        covered = {item.expected_code for item in fuzzer.scenario_catalog(1)}
        dispositions = fuzzer.rule_dispositions()
        test_backed = fuzzer.TEST_BACKED_RULES.keys()
        self.assertEqual((len(inventory), len(covered), len(inventory & covered)), (300, 306, 283))
        self.assertEqual(
            inventory - covered - dispositions.keys() - test_backed,
            {
                "FDA.G.k.12.COLLECTION.REQUIRED",
                "FDA.G.k.9.REQUIRED",
                "ICH.C.2.r.REQUIRED",
                "ICH.E.i.REQUIRED",
                "ICH.G.k.REQUIRED",
            },
        )
        self.assertEqual(dispositions.keys() & covered, set())
        self.assertEqual(test_backed & covered, set())

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
        self.assertEqual(len(generated), 306 * 3)
        self.assertTrue(all(item.generator_family in fuzzer.GENERATOR_FAMILIES for item in generated))
        self.assertEqual(len({item.generation_fingerprint for item in generated}), len(generated))

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
