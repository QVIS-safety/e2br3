use super::{PortableFieldBinding, PortableValueType};

macro_rules! binding {
	($section:literal, $path:literal, $request:literal, $type:ident, [$($code:literal),+ $(,)?]) => {
		PortableFieldBinding {
			section: $section,
			frontend_path: $path,
			request_path: $request,
			value_type: PortableValueType::$type,
			rule_codes: &[$($code),+],
			null_flavor_path: None,
		}
	};
	($section:literal, $path:literal, $request:literal, $type:ident, [$($code:literal),+ $(,)?], null: $null:literal) => {
		PortableFieldBinding {
			section: $section,
			frontend_path: $path,
			request_path: $request,
			value_type: PortableValueType::$type,
			rule_codes: &[$($code),+],
			null_flavor_path: Some($null),
		}
	};
}

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	binding!(
		"CI",
		"safetyReportIdentification.safetyReportId",
		"safetyReportId",
		String,
		["ICH.C.1.1.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.transmissionDate",
		"transmissionDate",
		String,
		["ICH.C.1.2.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.reportType",
		"reportType",
		String,
		["ICH.C.1.3.LENGTH.MAX", "ICH.C.1.3.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.dateFirstReceivedFromSource",
		"dateFirstReceivedFromSource",
		String,
		["ICH.C.1.4.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.dateOfMostRecentInformation",
		"dateOfMostRecentInformation",
		String,
		["ICH.C.1.5.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.additionalDocumentsAvailable",
		"additionalDocumentsAvailable",
		Boolean,
		["ICH.C.1.6.1.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.documentsHeldBySender[].documentDescription",
		"documentsHeldBySender[].documentDescription",
		String,
		["ICH.C.1.6.1.r.1.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.documentsHeldBySender[].includedDocument",
		"documentsHeldBySender[].includedDocument",
		String,
		["ICH.C.1.6.1.r.2.ALLOWED.VALUE"]
	),
	binding!("CI", "safetyReportIdentification.fulfilExpeditedCriteria", "fulfilExpeditedCriteria", Boolean, ["ICH.C.1.7.ALLOWED.VALUE"], null: "safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor"),
	binding!(
		"CI",
		"safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor",
		"fulfilExpeditedCriteriaNullFlavor",
		String,
		["ICH.C.1.7.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.localCriteriaReportType",
		"localCriteriaReportType",
		String,
		["FDA.C.1.7.1.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.worldwideUniqueId",
		"worldwideUniqueId",
		String,
		["ICH.C.1.8.1.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.firstSenderType",
		"firstSenderType",
		String,
		["ICH.C.1.8.2.LENGTH.MAX", "ICH.C.1.8.2.ALLOWED.VALUE"]
	),
	binding!("CI", "safetyReportIdentification.otherCaseIdentifiersExist", "otherCaseIdentifiersExist", Boolean, ["ICH.C.1.9.1.ALLOWED.VALUE"], null: "safetyReportIdentification.otherCaseIdentifiersExistNullFlavor"),
	binding!(
		"CI",
		"safetyReportIdentification.otherCaseIdentifiersExistNullFlavor",
		"otherCaseIdentifiersExistNullFlavor",
		String,
		["ICH.C.1.9.1.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.otherCaseIdentifiers[].source",
		"otherCaseIdentifiers[].source",
		String,
		["ICH.C.1.9.1.r.1.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.otherCaseIdentifiers[].caseIdentifier",
		"otherCaseIdentifiers[].caseIdentifier",
		String,
		["ICH.C.1.9.1.r.2.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.linkedReports[].linkedReportNumber",
		"linkedReports[].linkedReportNumber",
		String,
		["ICH.C.1.10.r.LENGTH.MAX"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.nullificationAmendmentCode",
		"nullificationAmendmentCode",
		String,
		["ICH.C.1.11.1.LENGTH.MAX", "ICH.C.1.11.1.ALLOWED.VALUE"]
	),
	binding!(
		"CI",
		"safetyReportIdentification.nullificationReason",
		"nullificationReason",
		String,
		["ICH.C.1.11.2.LENGTH.MAX"]
	),
	binding!("RP", "primarySources[].reporterTitle", "reporterTitle", String, ["ICH.C.2.r.1.1.LENGTH.MAX"], null: "primarySources[].reporterTitleNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterTitleNullFlavor",
		"reporterTitleNullFlavor",
		String,
		["ICH.C.2.r.1.1.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterGivenName", "reporterGivenName", String, ["ICH.C.2.r.1.2.LENGTH.MAX"], null: "primarySources[].reporterGivenNameNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterGivenNameNullFlavor",
		"reporterGivenNameNullFlavor",
		String,
		["ICH.C.2.r.1.2.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterMiddleName", "reporterMiddleName", String, ["ICH.C.2.r.1.3.LENGTH.MAX"], null: "primarySources[].reporterMiddleNameNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterMiddleNameNullFlavor",
		"reporterMiddleNameNullFlavor",
		String,
		["ICH.C.2.r.1.3.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterFamilyName", "reporterFamilyName", String, ["ICH.C.2.r.1.4.LENGTH.MAX"], null: "primarySources[].reporterFamilyNameNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterFamilyNameNullFlavor",
		"reporterFamilyNameNullFlavor",
		String,
		["ICH.C.2.r.1.4.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterOrganization", "reporterOrganization", String, ["ICH.C.2.r.2.1.LENGTH.MAX"], null: "primarySources[].reporterOrganizationNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterOrganizationNullFlavor",
		"reporterOrganizationNullFlavor",
		String,
		["ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterDepartment", "reporterDepartment", String, ["ICH.C.2.r.2.2.LENGTH.MAX"], null: "primarySources[].reporterDepartmentNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterDepartmentNullFlavor",
		"reporterDepartmentNullFlavor",
		String,
		["ICH.C.2.r.2.2.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterStreet", "reporterStreet", String, ["ICH.C.2.r.2.3.LENGTH.MAX"], null: "primarySources[].reporterStreetNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterStreetNullFlavor",
		"reporterStreetNullFlavor",
		String,
		["ICH.C.2.r.2.3.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterCity", "reporterCity", String, ["ICH.C.2.r.2.4.LENGTH.MAX"], null: "primarySources[].reporterCityNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterCityNullFlavor",
		"reporterCityNullFlavor",
		String,
		["ICH.C.2.r.2.4.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterState", "reporterState", String, ["ICH.C.2.r.2.5.LENGTH.MAX"], null: "primarySources[].reporterStateNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterStateNullFlavor",
		"reporterStateNullFlavor",
		String,
		["ICH.C.2.r.2.5.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterPostcode", "reporterPostcode", String, ["ICH.C.2.r.2.6.LENGTH.MAX"], null: "primarySources[].reporterPostcodeNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterPostcodeNullFlavor",
		"reporterPostcodeNullFlavor",
		String,
		["ICH.C.2.r.2.6.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterTelephone", "reporterTelephone", String, ["ICH.C.2.r.2.7.LENGTH.MAX"], null: "primarySources[].reporterTelephoneNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterTelephoneNullFlavor",
		"reporterTelephoneNullFlavor",
		String,
		["ICH.C.2.r.2.7.NULLFLAVOR.ALLOWED"]
	),
	binding!("RP", "primarySources[].reporterCountry", "reporterCountry", String, ["ICH.C.2.r.3.LENGTH.MAX"], null: "primarySources[].reporterCountryNullFlavor"),
	binding!(
		"RP",
		"primarySources[].reporterCountryNullFlavor",
		"reporterCountryNullFlavor",
		String,
		["ICH.C.2.r.3.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"RP",
		"primarySources[].reporterEmail",
		"reporterEmail",
		String,
		["FDA.C.2.r.2.8.LENGTH.MAX"]
	),
	binding!("RP", "primarySources[].qualification", "qualification", String, ["ICH.C.2.r.4.LENGTH.MAX", "ICH.C.2.r.4.ALLOWED.VALUE"], null: "primarySources[].qualificationNullFlavor"),
	binding!(
		"RP",
		"primarySources[].qualificationNullFlavor",
		"qualificationNullFlavor",
		String,
		["ICH.C.2.r.4.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"RP",
		"primarySources[].qualificationKr1",
		"qualificationKr1",
		String,
		["MFDS.C.2.r.4.KR.1.LENGTH.MAX"]
	),
	binding!(
		"RP",
		"primarySources[].primarySourceForRegulatoryPurposes",
		"primarySourceForRegulatoryPurposes",
		String,
		["ICH.C.2.r.5.LENGTH.MAX", "ICH.C.2.r.5.ALLOWED.VALUE"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderType",
		"senderType",
		String,
		["ICH.C.3.1.LENGTH.MAX", "ICH.C.3.1.ALLOWED.VALUE"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderHealthProfessionalTypeKr1",
		"senderHealthProfessionalTypeKr1",
		String,
		["MFDS.C.3.1.KR.1.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderOrganization",
		"senderOrganization",
		String,
		["ICH.C.3.2.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderDepartment",
		"senderDepartment",
		String,
		["ICH.C.3.3.1.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderPersonTitle",
		"senderPersonTitle",
		String,
		["ICH.C.3.3.2.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderPersonGivenName",
		"senderPersonGivenName",
		String,
		["ICH.C.3.3.3.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderPersonMiddleName",
		"senderPersonMiddleName",
		String,
		["ICH.C.3.3.4.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderPersonFamilyName",
		"senderPersonFamilyName",
		String,
		["ICH.C.3.3.5.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderStreetAddress",
		"senderStreetAddress",
		String,
		["ICH.C.3.4.1.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderCity",
		"senderCity",
		String,
		["ICH.C.3.4.2.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderState",
		"senderState",
		String,
		["ICH.C.3.4.3.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderPostcode",
		"senderPostcode",
		String,
		["ICH.C.3.4.4.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderCountryCode",
		"senderCountryCode",
		String,
		["ICH.C.3.4.5.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderTelephone",
		"senderTelephone",
		String,
		["ICH.C.3.4.6.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderFax",
		"senderFax",
		String,
		["ICH.C.3.4.7.LENGTH.MAX"]
	),
	binding!(
		"SD",
		"safetyReportIdentification.senderEmail",
		"senderEmail",
		String,
		["ICH.C.3.4.8.LENGTH.MAX"]
	),
	binding!("LR", "literatureReferences[].referenceText", "literatureReference", String, ["ICH.C.4.r.1.LENGTH.MAX"], null: "literatureReferences[].referenceTextNullFlavor"),
	binding!(
		"LR",
		"literatureReferences[].referenceTextNullFlavor",
		"referenceTextNullFlavor",
		String,
		["ICH.C.4.r.1.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"LR",
		"literatureReferences[].documentBase64",
		"documentBase64",
		String,
		["ICH.C.4.r.2.ALLOWED.VALUE"]
	),
	binding!("SI", "studyInformation.studyRegistrationNumbers[].registrationNumber", "studyRegistrationNumbers[].registrationNumber", String, ["ICH.C.5.1.r.1.LENGTH.MAX"], null: "studyInformation.studyRegistrationNumbers[].registrationNumberNullFlavor"),
	binding!(
		"SI",
		"studyInformation.studyRegistrationNumbers[].registrationNumberNullFlavor",
		"studyRegistrationNumbers[].registrationNumberNullFlavor",
		String,
		["ICH.C.5.1.r.1.NULLFLAVOR.ALLOWED"]
	),
	binding!("SI", "studyInformation.studyRegistrationNumbers[].countryCode", "studyRegistrationNumbers[].countryCode", String, ["ICH.C.5.1.r.2.LENGTH.MAX"], null: "studyInformation.studyRegistrationNumbers[].countryCodeNullFlavor"),
	binding!(
		"SI",
		"studyInformation.studyRegistrationNumbers[].countryCodeNullFlavor",
		"studyRegistrationNumbers[].countryCodeNullFlavor",
		String,
		["ICH.C.5.1.r.2.NULLFLAVOR.ALLOWED"]
	),
	binding!("SI", "studyInformation.studyName", "studyName", String, ["ICH.C.5.2.LENGTH.MAX"], null: "studyInformation.studyNameNullFlavor"),
	binding!(
		"SI",
		"studyInformation.studyNameNullFlavor",
		"studyNameNullFlavor",
		String,
		["ICH.C.5.2.NULLFLAVOR.ALLOWED"]
	),
	binding!("SI", "studyInformation.sponsorStudyNumber", "sponsorStudyNumber", String, ["ICH.C.5.3.LENGTH.MAX"], null: "studyInformation.sponsorStudyNumberNullFlavor"),
	binding!(
		"SI",
		"studyInformation.sponsorStudyNumberNullFlavor",
		"sponsorStudyNumberNullFlavor",
		String,
		["ICH.C.5.3.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"SI",
		"studyInformation.studyTypeReaction",
		"studyTypeReaction",
		String,
		["ICH.C.5.4.LENGTH.MAX", "ICH.C.5.4.ALLOWED.VALUE"]
	),
	binding!(
		"SI",
		"studyInformation.studyTypeReactionKr1",
		"studyTypeReactionKr1",
		String,
		["MFDS.C.5.4.KR.1.LENGTH.MAX"]
	),
	binding!(
		"SI",
		"studyInformation.fdaIndNumberOccurred",
		"fdaIndNumberOccurred",
		String,
		["FDA.C.5.5a.LENGTH.MAX"]
	),
	binding!(
		"SI",
		"studyInformation.fdaPreAndaNumberOccurred",
		"fdaPreAndaNumberOccurred",
		String,
		["FDA.C.5.5b.LENGTH.MAX"]
	),
	binding!(
		"SI",
		"studyInformation.fdaCrossReportedIndNumbers[].indNumber",
		"fdaCrossReportedIndNumbers[].indNumber",
		String,
		["FDA.C.5.6.r.LENGTH.MAX"]
	),
];
