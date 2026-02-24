use lib_core::xml::import_sections::g_drug::parse_g_drugs;
use lib_core::xml::Error as XmlError;

#[test]
fn import_g_drug_basic() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drugs = parse_g_drugs(&xml).expect("parse");
	assert!(!drugs.is_empty());
}

#[test]
fn import_g_drug_requires_medicinal_product_name() {
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
                    <organizer classCode="CATEGORY" moodCode="EVN">
                      <code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
                      <component typeCode="COMP">
                        <substanceAdministration classCode="SBADM" moodCode="EVN">
                          <id root="22222222-2222-2222-2222-222222222222"/>
                          <consumable typeCode="CSM">
                            <instanceOfKind classCode="INST">
                              <kindOfProduct classCode="MMAT" determinerCode="KIND"/>
                            </instanceOfKind>
                          </consumable>
                        </substanceAdministration>
                      </component>
                    </organizer>
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

	let err =
		parse_g_drugs(xml).expect_err("missing medicinal product name should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.G.k.2.2.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
