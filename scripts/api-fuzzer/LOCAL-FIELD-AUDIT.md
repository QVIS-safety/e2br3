# Local-field XML boundary audit (2026-11-26)

## Inventory and meaning

The case registry has 403 entries: 265 standard elements and 138 entries marked
`local_only`. That flag means “not an independent dictionary element”, **not**
“must never appear in XML”. The presave registry separately has 171 entries,
including 105 local-marked entries. `local-field-audit.json` records every local
entry with its model/field; a regression test rejects additions, removals or
mapping changes until the inventory is reviewed. It is an audit baseline, not
permission to skip all fields with a local-looking name.

| Case-registry group | Count | XML treatment |
| --- | ---: | --- |
| nullFlavor companions | 70 | Keep standard nullFlavor attributes and test their preservation. |
| XML components | 25 | Keep attachment ED attributes, identifier discriminators, code systems/versions, result qualifiers and FDA device characteristics. |
| App-only scalar values | 25 | Do not export; separately test persistence/non-disclosure. Includes 17 MFDS device UI values. |
| App-only source-document properties | 3 | Do not export this parallel upload collection; standard included documents remain exportable. |
| Review workflow controls | 3 | Internal review receiver/dates, not an E2B data element. Not in the 11-page fuzzer contracts. |
| Master-data link | 1 | Product presave UUID; target selection is validated independently, not copied through XML. |
| Legacy assessment value | 1 | `reaction_recurred` is not the official `recurrence_action`; retain local storage, do not export. |
| Derived seriousness | 1 | Derived from clinical seriousness criteria; retain comparison of the derived result, not a blanket exemption. |
| Inactive MFDS drug-device UI inventory | 9 | Backend mapping marked missing; no current editor contract or XML writer. Frontend explicitly removes `mfdsDeviceInfo`. Not claimed as populated coverage. |

The eight non-MFDS-device scalar values are `reportYear`, `fdaReportType`,
`mfdsReportType`, narrative `additional_information`, drug `batch_lot_number`,
dosage `continuing`, and assessment `expectedness` / `dechallengeResult`.

## Product defect found and fixed

`crates/libs/xml/src/export/sections/e.rs` emitted 17 local MFDS device UI values
as `KR_DVC_*` observations without a standard code-system mapping. The case
registry explicitly marks these as local and says an official dictionary source
is missing. The pinned MFDS dictionary has 14 regional elements and no section-E
device extensions. `mapping/mfds/e_reaction.rs` has an empty `KR_FIELDS` list.
The export writer nevertheless had its own private extension block.

That block and its three unused helpers are removed. A fixed XML-crate test fills
all 17 fields (including true and false booleans), checks unchanged output under
ICH/FDA/MFDS, tests the patch writer, and checks standard reaction/NullFlavor
preservation. DB columns, editor inputs, and legacy import readers are retained;
old data is not deleted. Legacy import acceptance is not permission to re-export
the private extension.

Official MFDS resources distinguish the [medical-device adverse-event reporting
form](https://emedi.mfds.go.kr/msismext/emd/rpt/abcsRpt/abcrptReportView.do) from the
[drug adverse-event E2B reporting system](https://nedrug.mfds.go.kr/minwon/adverse).
The medical-device form supports these clinical/workflow concepts, but is not
evidence that our invented `KR_DVC_*` XML codes are standard. No new standard
mapping is asserted by this correction.

## Existing correct export boundaries retained

- Source documents: no source-document query in the outbound exporter; standard
  `DocumentsHeldBySender` / literature ED attachments remain separate.
- Product UUID/display ID and dosage-continuing PIs: existing XML-crate fixed
  tests in `export/sections/g.rs` verify non-disclosure; legacy readers remain.
- Drug-level batch lot: existing fixed test preserves official dosage/device
  lots while suppressing the local convenience field.
- Case year/classifications/workflow: `export/sections/c.rs` selects the safety
  report fields, not these Case properties. Official FDA `localCriteriaReportType`
  (`FDA.C.1.7.1`) remains exported.
- Additional narrative and assessment expectedness/dechallenge/legacy recurrence:
  no outbound writer; official H values and `recurrence_action` remain exported.
- `E.local.expectedness` and `E.local.severity` are already absent from the
  current case registry/model/export writer; drug-assessment expectedness is a
  different, still-used local field.

## Internal references and master data

Audited ID-bearing paths across the saved 100-case API snapshots and the editor
projections. Parent foreign keys (including `death_info_id` and `device_id`) are
validated against the remapped target parent. Row IDs/audit metadata are not
clinical values. Native reaction/drug IDs are also used as standard XML row IDs
and relationship targets; they must not be stripped indiscriminately. Imported
XML identities are preserved by `export/xml_identity.rs`.

Sender/product presave links require independently expected target selections.
Reporter/study/narrative presave links and `sourceProductId` are local metadata,
not clinical fields. Their raw differences remain visible. Standard report IDs,
MPID/PhPID, patient identifiers and nullFlavor are not exempted by suffix/name.

Presave local entries include master descriptions, templates, deletion/default
flags, routing/due-date configuration, child IDs and standard-field companions.
The XML crate reads case-level models, not whole presave records. Selected master
values may intentionally become standard case fields or outbound header
identifiers; this does not make all master data exportable. Header output follows
explicit `OutboundMessageHeader` values in `export/sections/n.rs`. Download and
submission both call `export_case_xml_with_options`; notation only controls XML
comments, not master-data notation fields. No separate local-field serialization
path was found in those entry points.

## Fuzzer correction and limits

26 of the current 353 editor-contract fields are local-only: the 25 scalar
values plus the source-document name contract (whose generated row also checks
payload/MIME persistence). They no longer count as XML-preservation coverage.
Every populated scalar is cleared through the real API, read back, exported and
compared byte-for-byte with the original XML, then restored in `finally`.
Nested dosage/assessment probes address existing child IDs; they must not delete
or recreate rows. Source values and headers must be unchanged after restoration.
`localFieldChecks` and `checkedLocalOnlyFields` record this separately. Missing
coverage remains a campaign failure. Raw before/after differences are retained.

The prior `reporterEmailNullFlavor` coverage gap was a separate harness bug:
the generated reporter rows intentionally separate an email value from its
nullFlavor. Projecting the sparse input across all rows threw on the other row's
absent key and skipped the whole field. `plannedValues` now checks each explicitly
provided value at its original array indexes, without inventing missing values.
The observed source/target arrays still undergo full comparison.

The comparator's allowlist is path-specific and type-checked: missing fields,
invalid values, missing parents, standard-value loss and array changes still
fail. The runner checks that its audited contract paths agree with the comparator.

Inventory/source inspection is not a claim of live populated coverage for all
403 registry entries, all 171 master fields, review workflows, inactive UI fields,
or every possible XML input. The live fuzzer uses an isolated DB and synthetic
terminology fixtures; it is not licensed terminology or regulatory submission
certification. Run artifacts, not this inventory, establish executed coverage.

## Executed validation

- Real HTTP API + PostgreSQL campaign, seed `20261123`: **100 cases × 5 hops =
  500 transfers**, 37,549 requests, zero failures. Standard XML coverage is
  **327/327** contracts; separate local persistence/non-disclosure coverage is
  **26/26**. No contract remains unchecked.
- Independently rechecked all 500 saved XML files for private markers and all
  500 comparison results. All 98 scalar clear/export/restore probes have identical
  before/without-local XML and successful restoration records. Source-document
  checks are recorded separately. All 15 recorded source hashes still match.
- `cargo test -p xml --lib --test import --test xml`: **315 passed**.
- Runner/comparator/preservation Node tests: **40 passed**. Registry export-writer
  coverage unittest: **1 passed**. Scoped diff whitespace checks passed.
- The historical `recheck-four-fixes-20261126` campaign's first-hop XML contains
  the now-removed private MFDS markers in **33/100** files. The new structural
  guard rejects those files; a successful old round trip alone did not establish
  non-disclosure.

Evidence: [campaign report](../../tmp/api-fuzzer/local-field-audit-100-20261126/report.json)
and the adjacent XML, snapshots, comparisons, plans and request log. The isolated
database and evidence are retained; the task-specific server/container are stopped
after verification. No shared application database was reset.
