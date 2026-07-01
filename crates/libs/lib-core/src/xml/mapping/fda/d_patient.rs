// FDA mapping for Section D (Patient).

pub struct DPatientPaths;

impl DPatientPaths {
	// D.1 Patient initials/name
	pub const PATIENT_NAME: &'static str = "//hl7:primaryRole/hl7:player1/hl7:name";
	pub const PATIENT_NAME_NULL_FLAVOR: &'static str =
		"//hl7:primaryRole/hl7:player1/hl7:name/@nullFlavor";

	// D.5 Sex
	pub const SEX_CODE: &'static str =
		"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode/@code";
	pub const SEX_NULL_FLAVOR: &'static str =
		"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode/@nullFlavor";

	// D.2.1 Date of Birth
	pub const BIRTH_DATE: &'static str =
		"//hl7:primaryRole/hl7:player1/hl7:birthTime/@value";
	pub const BIRTH_DATE_NULL_FLAVOR: &'static str =
		"//hl7:primaryRole/hl7:player1/hl7:birthTime/@nullFlavor";

	// D.2.2 Age at Time of Onset (value/unit)
	pub const AGE_VALUE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
	pub const AGE_UNIT: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@unit";
	pub const AGE_NULL_FLAVOR: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@nullFlavor";

	// D.2.2.1 Gestation period (value/unit)
	pub const GESTATION_VALUE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='16' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
	pub const GESTATION_UNIT: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='16' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@unit";

	// D.2.3 Age Group
	pub const AGE_GROUP_CODE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@code";

	// D.3 Weight
	pub const WEIGHT_VALUE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='7' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";

	// D.4 Height
	pub const HEIGHT_VALUE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='17' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";

	// FDA.D.11 Race
	pub const RACE_CODE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@code";
	pub const RACE_CODE_NULL_FLAVOR: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@nullFlavor";

	// FDA.D.12 Ethnicity
	pub const ETHNICITY_CODE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@code";
	pub const ETHNICITY_CODE_NULL_FLAVOR: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@nullFlavor";

	// D.6 Last Menstrual Period
	pub const LMP_DATE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='22' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
	pub const LMP_DATE_NULL_FLAVOR: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='22' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@nullFlavor";

	// D.7.2 Medical history text
	pub const MEDICAL_HISTORY_TEXT: &'static str =
		"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation[hl7:code[@code='18' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value";

	// D.7.3 Concomitant therapy
	pub const CONCOMITANT_THERAPY_VALUE: &'static str =
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='28' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
}
