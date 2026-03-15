// FDA mapping for Section C (Safety Report Identification).
// Source: ICH ICSR Technical Information (Nov 2016) + FDA regional rules.

pub struct CSafetyReportPaths;

impl CSafetyReportPaths {
	// C.1.1 Sender's (case) Safety Report Unique Identifier
	pub const REPORT_UNIQUE_ID_EXT: &'static str =
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension";

	// C.1.2 Date of Creation
	pub const DATE_OF_CREATION: &'static str =
		"//hl7:controlActProcess/hl7:effectiveTime/@value";

	// C.1.3 Type of Report
	pub const TYPE_OF_REPORT_CODE: &'static str =
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code";

	// C.1.4 Date Report Was First Received from Source
	pub const DATE_FIRST_RECEIVED: &'static str =
		"//hl7:investigationEvent/hl7:effectiveTime/hl7:low/@value";

	// C.1.5 Date of Most Recent Information for This Report
	pub const DATE_MOST_RECENT: &'static str =
		"//hl7:investigationEvent/hl7:availabilityTime/@value";

	// C.1.7 Does This Case Fulfil the Local Criteria for an Expedited Report?
	pub const FULFIL_EXPEDITED: &'static str =
		"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";

	// C.1.6.1 Are Additional Documents Available?
	pub const ADDITIONAL_DOCUMENTS_AVAILABLE: &'static str =
		"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";

	// C.1.8.1 Worldwide Unique Case Identification Number
	pub const WORLDWIDE_UNIQUE_ID_EXT: &'static str =
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']/@extension";

	// C.1.8.2 First Sender of This Case
	pub const FIRST_SENDER_TYPE: &'static str =
		"//hl7:outboundRelationship[@typeCode='SPRT'][hl7:relatedInvestigation/hl7:code[@code='1']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code";

	// C.1.11.1 Nullification/Amendment Code
	pub const NULLIFICATION_CODE: &'static str =
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code";

	// C.1.11.2 Nullification/Amendment Reason
	pub const NULLIFICATION_REASON: &'static str =
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/hl7:originalText";

	// FDA.C.1.7.1 Local Criteria Report Type (FDA)
	pub const FDA_LOCAL_CRITERIA_REPORT_TYPE_CODE: &'static str =
		"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@code";

	// FDA.C.1.12 Combination Product Report Indicator (FDA)
	pub const FDA_COMBINATION_PRODUCT_INDICATOR_VALUE: &'static str =
		"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@value";
}
