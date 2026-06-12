import unittest
import json
import tempfile
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extract_frontend_fields as extractor


class FrontendFieldExtractorTests(unittest.TestCase):
    def test_normalizes_template_repeatable_indexes(self):
        self.assertEqual(
            "reactions.reactionCountry",
            extractor.normalize_field_path("reactions.${activeIndex}.reactionCountry"),
        )
        self.assertEqual(
            "patientInformation.medicalHistoryEpisodes.comments",
            extractor.normalize_field_path(
                "patientInformation.medicalHistoryEpisodes.${index}.comments"
            ),
        )
        self.assertEqual(
            "drugs.indications.indicationText",
            extractor.extract_field_paths_from_source(
                '<Controller name={`drugs.${index}.indications.${indicationIndex}.indicationText`} />'
            )[0],
        )
        self.assertEqual(
            "drugs.drugReactionAssessments.reactionId",
            extractor.extract_field_paths_from_source(
                '<Controller name={`drugs.${index}.drugReactionAssessments.${assessIndex}.reactionId`} />'
            )[0],
        )

    def test_normalizes_numeric_repeatable_indexes(self):
        self.assertEqual(
            "testResults.comments",
            extractor.normalize_field_path("testResults.0.comments"),
        )

    def test_preserves_business_field_names(self):
        self.assertEqual(
            "reactions.reactionCountry",
            extractor.normalize_field_path("reactions.${activeIndex}.reactionCountry"),
        )

    def test_extracts_literal_and_template_name_props(self):
        source = '''
<Controller name="patientInformation.patientAge.value" control={control} />
<Controller
  name={`reactions.${activeIndex}.reactionCountry`}
  control={control}
/>
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual(
            [
                "patientInformation.patientAge.value",
                "reactions.reactionCountry",
            ],
            fields,
        )

    def test_extracts_register_calls(self):
        source = '''
<Input {...register("safetyReportIdentification.receiverEmail")} />
<Input {...register(`testResults.${index}.comments`)} />
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual(
            [
                "safetyReportIdentification.receiverEmail",
                "testResults.comments",
            ],
            fields,
        )

    def test_ignores_unresolved_template_variable_paths_and_e2b_codes(self):
        source = '''
<Controller name={`${name}.${index}.${valueKey}`} />
<Controller name={`reactions.${activeIndex}.mfdsDeviceAe.${name}`} />
<E2BFormField fieldNumber="C.2.r.5" />
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual([], fields)

    def test_expands_string_tuple_map_field_names(self):
        source = '''
{[
  ["actionRecall", "KR_DVC_ACT_RC", "회수"],
  ["actionRepair", "KR_DVC_ACT_RP", "수리"],
].map(([name, fieldNumber, label]) => (
  <Controller name={`reactions.${activeIndex}.mfdsDeviceAe.${name}`} />
))}
'''

        fields = extractor.extract_field_paths_from_source(source)

        self.assertEqual(
            [
                "reactions.mfdsDeviceAe.actionRecall",
                "reactions.mfdsDeviceAe.actionRepair",
            ],
            fields,
        )

    def test_extracts_inventory_from_configured_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            frontend = root / "frontend"
            frontend.mkdir()
            section = frontend / "SectionE.tsx"
            section.write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )

            fields = extractor.extract_frontend_fields(
                root=root,
                source_globs=["frontend/SectionE.tsx"],
            )

        self.assertEqual(1, len(fields))
        self.assertEqual("reactions.reactionCountry", fields[0].key)
        self.assertEqual("reactions", fields[0].section)
        self.assertEqual("reactionCountry", fields[0].field)
        self.assertEqual("reactions.${activeIndex}.reactionCountry", fields[0].raw)

    def test_inventory_paths_resolve_from_repo_root_when_registry_root_is_nested(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo_root = Path(tmp)
            registry_root = repo_root / "registry"
            registry_root.mkdir()
            frontend = repo_root / "frontend"
            frontend.mkdir()
            section = frontend / "SectionE.tsx"
            section.write_text(
                '<Controller name={`reactions.${activeIndex}.reactionCountry`} />',
                encoding="utf-8",
            )

            fields = extractor.extract_frontend_fields(
                root=registry_root,
                source_globs=["frontend/SectionE.tsx"],
            )

        self.assertEqual("reactions.reactionCountry", fields[0].key)

    def test_missing_glob_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(extractor.FrontendInventoryError):
                extractor.extract_frontend_fields(
                    root=Path(tmp),
                    source_globs=["frontend/Missing.tsx"],
                )

    def test_json_output_is_deterministic(self):
        field = extractor.FrontendField(
            key="reactions.reactionCountry",
            section="reactions",
            field="reactionCountry",
            file="frontend/SectionE.tsx",
            raw="reactions.${activeIndex}.reactionCountry",
        )

        payload = extractor.fields_to_json([field])

        self.assertEqual(
            [
                {
                    "key": "reactions.reactionCountry",
                    "section": "reactions",
                    "field": "reactionCountry",
                    "file": "frontend/SectionE.tsx",
                    "raw": "reactions.${activeIndex}.reactionCountry",
                }
            ],
            json.loads(payload),
        )


if __name__ == "__main__":
    unittest.main()
