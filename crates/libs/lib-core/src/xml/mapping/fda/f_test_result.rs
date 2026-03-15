// FDA mapping for Section F (Tests and Procedures).

pub struct FTestResultPaths;

impl FTestResultPaths {
	pub const TEST_NODE: &'static str =
		"//hl7:organizer[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation";

	pub const TEST_NAME: &'static str = "hl7:code/hl7:originalText";
	pub const TEST_NAME_DISPLAY: &'static str = "hl7:code/@displayName";
	pub const TEST_MEDDRA_CODE: &'static str = "hl7:code/@code";
	pub const TEST_MEDDRA_VERSION: &'static str = "hl7:code/@codeSystemVersion";
	pub const TEST_DATE: &'static str = "hl7:effectiveTime/@value";
	pub const TEST_DATE_NULL_FLAVOR: &'static str = "hl7:effectiveTime/@nullFlavor";
	pub const RESULT_CODE: &'static str = "hl7:interpretationCode/@code";
	pub const RESULT_VALUE: &'static str = "hl7:value/hl7:center/@value";
	pub const RESULT_VALUE_FALLBACK: &'static str = "hl7:value/@value";
	pub const RESULT_UNIT: &'static str = "hl7:value/hl7:center/@unit";
	pub const RESULT_UNIT_FALLBACK: &'static str = "hl7:value/@unit";
	pub const RESULT_UNSTRUCTURED: &'static str = "hl7:value";
	pub const NORMAL_LOW: &'static str =
		"hl7:referenceRange/hl7:observationRange[hl7:interpretationCode[@code='L']]/hl7:value/@value";
	pub const NORMAL_HIGH: &'static str =
		"hl7:referenceRange/hl7:observationRange[hl7:interpretationCode[@code='H']]/hl7:value/@value";
	pub const COMMENTS: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value";
	pub const MORE_INFO: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='11']]/hl7:value/@value";
}
