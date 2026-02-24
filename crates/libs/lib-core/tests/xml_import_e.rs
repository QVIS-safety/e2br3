use lib_core::xml::import_sections::e_reaction::parse_e_reactions;
use lib_core::xml::Error as XmlError;

#[test]
fn import_e_reaction_basic() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let reactions = parse_e_reactions(&xml).expect("parse");
	assert!(!reactions.is_empty());
}

#[test]
fn import_e_reaction_requires_primary_text() {
	let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject typeCode="SUBJ">
        <investigationEvent classCode="INVSTG" moodCode="EVN">
          <component typeCode="COMP">
            <adverseEventAssessment classCode="INVSTG" moodCode="EVN">
              <subject1 typeCode="SBJ">
                <primaryRole classCode="INVSBJ">
                  <subjectOf2 typeCode="SBJ">
                    <observation classCode="OBS" moodCode="EVN">
                      <id root="11111111-1111-1111-1111-111111111111"/>
                      <code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/>
                      <value xsi:type="CE" code="10027940" codeSystem="2.16.840.1.113883.6.163"/>
                    </observation>
                  </subjectOf2>
                </primaryRole>
              </subject1>
            </adverseEventAssessment>
          </component>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let err = parse_e_reactions(xml).expect_err("missing reaction text should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.E.i.1.1a.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
