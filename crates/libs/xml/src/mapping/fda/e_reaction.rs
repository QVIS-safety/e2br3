// FDA mapping for Section E (Reaction/Event).

pub struct EReactionPaths;

const REACTION_DURATION_UNITS: &[(&str, &str)] = &[
	("800", "10.a"),
	("801", "a"),
	("802", "mo"),
	("803", "wk"),
	("804", "d"),
	("805", "h"),
];

pub(crate) fn reaction_duration_unit_to_ucum(code: &str) -> Option<&'static str> {
	REACTION_DURATION_UNITS.iter().find_map(|(stored, ucum)| {
		(*stored == code || *ucum == code).then_some(*ucum)
	})
}

pub(crate) fn reaction_duration_unit_from_ucum(unit: &str) -> Option<&'static str> {
	REACTION_DURATION_UNITS
		.iter()
		.find_map(|(stored, ucum)| (*ucum == unit).then_some(*stored))
}

impl EReactionPaths {
	// Base reaction observation node
	pub const REACTION_NODE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]";

	pub const XML_ID_ROOT: &'static str = "hl7:id/@root";
	pub const PRIMARY_TEXT: &'static str =
		"hl7:value[@xsi:type='CE']/hl7:originalText";
	pub const PRIMARY_TEXT_ALT: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='30']]/hl7:value";
	pub const TRANSLATION_TEXT: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='30']]/hl7:value";
	pub const MEDDRA_CODE: &'static str = "hl7:value[@xsi:type='CE']/@code";
	pub const MEDDRA_VERSION: &'static str =
		"hl7:value[@xsi:type='CE']/@codeSystemVersion";
	pub const PRIMARY_LANG: &'static str =
		"hl7:value[@xsi:type='CE']/hl7:originalText/@language";

	// Term highlighted / seriousness
	pub const TERM_HIGHLIGHT_CODE: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='37']]/hl7:value/@code";

	// Seriousness criteria
	pub const CRITERIA_DEATH: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='34']]/hl7:value/@value";
	pub const CRITERIA_DEATH_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='34']]/hl7:value/@nullFlavor";
	pub const CRITERIA_LIFE_THREATENING: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='21']]/hl7:value/@value";
	pub const CRITERIA_LIFE_THREATENING_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='21']]/hl7:value/@nullFlavor";
	pub const CRITERIA_HOSPITALIZATION: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='33']]/hl7:value/@value";
	pub const CRITERIA_HOSPITALIZATION_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='33']]/hl7:value/@nullFlavor";
	pub const CRITERIA_DISABLING: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='35']]/hl7:value/@value";
	pub const CRITERIA_DISABLING_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='35']]/hl7:value/@nullFlavor";
	pub const CRITERIA_CONGENITAL: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='12']]/hl7:value/@value";
	pub const CRITERIA_CONGENITAL_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='12']]/hl7:value/@nullFlavor";
	pub const CRITERIA_OTHER: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='26']]/hl7:value/@value";
	pub const CRITERIA_OTHER_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='26']]/hl7:value/@nullFlavor";

	// FDA.E.i.3.2h Required Intervention
	pub const REQUIRED_INTERVENTION: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='7']]/hl7:value/@value";
	pub const REQUIRED_INTERVENTION_NULL_FLAVOR: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='7']]/hl7:value/@nullFlavor";

	// Dates / duration
	pub const START_DATE: &'static str =
		"hl7:effectiveTime/hl7:low/@value | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:low/@value";
	pub const START_DATE_NULL_FLAVOR: &'static str =
		"hl7:effectiveTime/hl7:low/@nullFlavor | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:low/@nullFlavor";
	pub const END_DATE: &'static str =
		"hl7:effectiveTime/hl7:high/@value | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:high/@value";
	pub const END_DATE_NULL_FLAVOR: &'static str =
		"hl7:effectiveTime/hl7:high/@nullFlavor | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:high/@nullFlavor";
	pub const DURATION_VALUE: &'static str =
		"hl7:effectiveTime/hl7:width/@value | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:width/@value";
	pub const DURATION_UNIT: &'static str =
		"hl7:effectiveTime/hl7:width/@unit | hl7:effectiveTime/hl7:comp[@xsi:type='IVL_TS']/hl7:width/@unit";

	// Outcome / confirmation / country
	pub const OUTCOME_CODE: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='27']]/hl7:value/@code";
	pub const MEDICAL_CONFIRMATION: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='24']]/hl7:value/@value";
	pub const COUNTRY_CODE: &'static str =
		"hl7:location/hl7:locatedEntity/hl7:locatedPlace/hl7:code/@code";
}
