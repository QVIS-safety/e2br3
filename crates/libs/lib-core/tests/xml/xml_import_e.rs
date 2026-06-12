use lib_core::xml::import_sections::e_reaction::parse_e_reactions;

#[test]
fn import_e_reaction_basic() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let reactions = parse_e_reactions(&xml).expect("parse");
	assert!(!reactions.is_empty());
}

#[test]
fn import_e_reaction_allows_missing_primary_text_for_validation_phase() {
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

	let reactions = parse_e_reactions(xml).expect("parse should succeed");
	assert_eq!(reactions.len(), 1);
	assert_eq!(reactions[0].primary_source_reaction, "");
}

#[test]
fn import_e_reaction_preserves_known_extension_fields() {
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
                      <value xsi:type="CE" code="10027940" codeSystem="2.16.840.1.113883.6.163">
                        <originalText>Device site pain</originalText>
                      </value>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="AE_IME_LIST"/>
                          <value xsi:type="BL" value="true"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="AE_EXPECTEDNESS"/>
                          <value xsi:type="CE" code="2"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="AE_SEVERITY"/>
                          <value xsi:type="CE" code="severe"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_AECL"/>
                          <value xsi:type="CE" code="0"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_AEOUT"/>
                          <value xsi:type="CE" code="10"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_CC_MD"/>
                          <value xsi:type="BL" value="true"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_CC_OTH"/>
                          <value xsi:type="ED">Other cause &amp; notes</value>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_ACT_RSN"/>
                          <value xsi:type="ED">Action reason text</value>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_ACT_RC"/>
                          <value xsi:type="BL" value="true"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_ACT_CAS"/>
                          <value xsi:type="BL" value="false"/>
                        </observation>
                      </outboundRelationship2>
                      <outboundRelationship2 typeCode="PERT">
                        <observation classCode="OBS" moodCode="EVN">
                          <code code="KR_DVC_ACT_OTH"/>
                          <value xsi:type="ED">Other action</value>
                        </observation>
                      </outboundRelationship2>
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

	let reactions = parse_e_reactions(xml).expect("parse should succeed");
	assert_eq!(reactions.len(), 1);
	let reaction = &reactions[0];
	assert_eq!(reaction.included_in_ema_ime_list, Some(true));
	assert_eq!(reaction.expectedness.as_deref(), Some("2"));
	assert_eq!(reaction.severity.as_deref(), Some("severe"));
	assert_eq!(reaction.mfds_device_ae_classification.as_deref(), Some("0"));
	assert_eq!(reaction.mfds_device_ae_outcome.as_deref(), Some("10"));
	assert_eq!(reaction.mfds_device_cause_medical_device, Some(true));
	assert_eq!(
		reaction.mfds_device_cause_other.as_deref(),
		Some("Other cause & notes")
	);
	assert_eq!(
		reaction.mfds_device_action_reason.as_deref(),
		Some("Action reason text")
	);
	assert_eq!(reaction.mfds_device_action_recall, Some(true));
	assert_eq!(reaction.mfds_device_action_label_change, Some(false));
	assert_eq!(
		reaction.mfds_device_action_other.as_deref(),
		Some("Other action")
	);
}
