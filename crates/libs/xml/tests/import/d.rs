use crate::common::{date, decimal, fixture};
use xml::import_sections::d_patient::parse_d_patient;

#[test]
fn import_d_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let patient = parse_d_patient(&xml)
		.expect("parse")
		.expect("section D should exist");

	assert_eq!(patient.patient_initials.as_deref(), Some("SM"));
	assert_eq!(patient.patient_initials_null_flavor, None);
	assert_eq!(patient.birth_date, Some(date(2014, 10, 1)));
	assert_eq!(patient.birth_date_null_flavor, None);
	assert_eq!(patient.sex.as_deref(), Some("1"));
	assert_eq!(patient.sex_null_flavor, None);
	// The fixture's parent age is 33; the patient's D.2.2a age is 10.
	assert_eq!(patient.age_at_time_of_onset, Some(decimal("10")));
	assert_eq!(patient.age_unit.as_deref(), Some("a"));
	assert_eq!(patient.gestation_period, Some(decimal("10")));
	assert_eq!(patient.gestation_period_unit.as_deref(), Some("wk"));
	assert_eq!(patient.age_group, None);
	assert_eq!(patient.weight_kg, Some(decimal("48.2")));
	assert_eq!(patient.height_cm, Some(decimal("152")));
	assert_eq!(
		patient.race_codes,
		["C16352", "C41259", "C41260", "C41219", "C41261"]
	);
	assert_eq!(patient.race_code_null_flavor, None);
	assert_eq!(patient.ethnicity_code.as_deref(), Some("C17459"));
	assert_eq!(patient.ethnicity_code_null_flavor, None);
	assert_eq!(patient.last_menstrual_period_date, Some(date(2009, 1, 1)));
	assert_eq!(patient.last_menstrual_period_date_null_flavor, None);
	assert_eq!(
		patient.medical_history_text.as_deref(),
		Some("The patient's medical history does not show anything very out of the ordinary.")
	);
	assert_eq!(patient.concomitant_therapy, Some(true));
}

#[test]
fn import_d_section_parses_race_ethnicity_null_flavor() {
	// FDA.D.11 / FDA.D.12 with nullFlavor instead of a coded value.
	let race_null = "UNK";
	let ethnicity_null = "UNK";
	let xml = format!(
		r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <primaryRole>
    <subjectOf2>
      <observation>
        <code code="C17049" codeSystem="2.16.840.1.113883.3.26.1.1"/>
        <value xsi:type="CE" nullFlavor="{race_null}"/>
      </observation>
    </subjectOf2>
    <subjectOf2>
      <observation>
        <code code="C16564" codeSystem="2.16.840.1.113883.3.26.1.1"/>
        <value xsi:type="CE" nullFlavor="{ethnicity_null}"/>
      </observation>
    </subjectOf2>
  </primaryRole>
</MCCI_IN200100UV01>"#
	);

	let patient = parse_d_patient(xml.as_bytes())
		.expect("parse")
		.expect("section D should exist when only a null flavor is present");

	assert!(patient.race_codes.is_empty());
	assert_eq!(patient.race_code_null_flavor.as_deref(), Some(race_null));
	assert_eq!(patient.ethnicity_code, None);
	assert_eq!(
		patient.ethnicity_code_null_flavor.as_deref(),
		Some(ethnicity_null)
	);
}

#[test]
fn import_d_section_preserves_false_concomitant_therapy() {
	let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <primaryRole>
    <subjectOf2>
      <organizer>
        <code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
        <component>
          <observation>
            <code code="11" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/>
            <value xsi:type="BL" value="false"/>
          </observation>
        </component>
      </organizer>
    </subjectOf2>
  </primaryRole>
</MCCI_IN200100UV01>"#;

	let patient = parse_d_patient(xml)
		.expect("parse")
		.expect("section D should exist");
	assert_eq!(patient.concomitant_therapy, Some(false));
}

#[test]
fn import_d_section_parses_repeated_race_codes() {
	let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <primaryRole>
    <subjectOf2><observation><code code="C17049" codeSystem="2.16.840.1.113883.3.26.1.1"/><value xsi:type="CE" code="C16352"/></observation></subjectOf2>
    <subjectOf2><observation><code code="C17049" codeSystem="2.16.840.1.113883.3.26.1.1"/><value xsi:type="CE" code="C41259"/></observation></subjectOf2>
  </primaryRole>
</MCCI_IN200100UV01>"#;

	let patient = parse_d_patient(xml)
		.expect("parse")
		.expect("race codes create a patient section");
	assert_eq!(patient.race_codes, ["C16352", "C41259"]);
	assert_eq!(patient.race_code_null_flavor, None);
}

#[test]
fn import_d_section_exists_when_only_birth_date_is_present() {
	let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><primaryRole><player1><birthTime value="19800102"/></player1></primaryRole></MCCI_IN200100UV01>"#;

	let patient = parse_d_patient(xml)
		.expect("parse")
		.expect("the patient node defines section D");

	assert_eq!(patient.birth_date, Some(date(1980, 1, 2)));
}
