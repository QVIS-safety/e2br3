use lib_core::xml::import_sections::f_test_result::parse_f_test_results;
use lib_core::xml::Error as XmlError;

#[test]
fn import_f_test_basic() {
	let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
	<PORR_IN049016UV>
		<controlActProcess>
			<subject>
				<investigationEvent>
					<component>
						<adverseEventAssessment>
							<subject1>
								<primaryRole>
									<subjectOf2>
										<organizer>
											<code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
											<component>
												<observation>
													<code code="10001552" displayName="ALT"/>
													<value value="25" unit="U/L"/>
												</observation>
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

	let _ = parse_f_test_results(xml).expect("parse");
}

#[test]
fn import_f_test_requires_test_name() {
	let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
	<PORR_IN049016UV>
		<controlActProcess>
			<subject>
				<investigationEvent>
					<component>
						<adverseEventAssessment>
							<subject1>
								<primaryRole>
									<subjectOf2>
										<organizer>
											<code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
											<component>
												<observation>
													<value value="25" unit="U/L"/>
												</observation>
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

	let err = parse_f_test_results(xml).expect_err("missing test name should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.F.r.2.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
