// FDA mapping for Section G (Drug/Biological).

pub struct GDrugPaths;

impl GDrugPaths {
	pub const DRUG_NODE: &'static str =
		"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration";

	pub const XML_ID_ROOT: &'static str = "hl7:id/@root";
	pub const PRODUCT_NAME_1: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[1]";
	pub const PRODUCT_NAME_2: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[2]";
	pub const MPID: &'static str =
		"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]";
	pub const MPID_VERSION: &'static str =
		"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]";
	pub const PHPID: &'static str =
		"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:id/@extension)[1]";
	pub const PHPID_VERSION: &'static str =
		"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:code/@codeSystemVersion)[1]";
	pub const INVESTIGATIONAL_BLINDED: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:subjectOf/hl7:observation[hl7:code[@code='G.k.2.5']]/hl7:value/@value";
	pub const DRUG_AUTHORIZATION_NUMBER: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:id/@extension";
	pub const MANUFACTURER_NAME: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:holder/hl7:role/hl7:playingOrganization/hl7:name";
	pub const MANUFACTURER_COUNTRY: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:author/hl7:territorialAuthority/hl7:territory/hl7:code/@code";
	pub const OBTAIN_DRUG_COUNTRY: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:subjectOf/hl7:productEvent/hl7:performer/hl7:assignedEntity/hl7:representedOrganization/hl7:addr/hl7:country";
	pub const ACTION_TAKEN: &'static str =
		"hl7:inboundRelationship[@typeCode='CAUS']/hl7:act/hl7:code/@code";
	pub const RECHALLENGE: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='31']]/hl7:value/@code";
	pub const BATCH_LOT_NUMBER: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText";
	pub const CUMULATIVE_DOSE_VALUE: &'static str =
		"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value/@value";
	pub const CUMULATIVE_DOSE_UNIT: &'static str =
		"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value/@unit";
	pub const GESTATION_EXPOSURE_VALUE: &'static str =
		"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value/@value";
	pub const GESTATION_EXPOSURE_UNIT: &'static str =
		"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value/@unit";
	pub const FDA_ADDITIONAL_INFO: &'static str =
		"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='9']]/hl7:value/@code";
	pub const FDA_SPECIALIZED_PRODUCT_CATEGORY: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@displayName='FDA Specialized Product Category']]/hl7:value/@code";
	pub const PARENT_ROUTE_TERMID_VERSION: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@codeSystemVersion";
	pub const PARENT_ROUTE_TERMID: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@code";
	pub const PARENT_DOSAGE_TEXT: &'static str =
		"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='2']]/hl7:value";
	pub const PARENT_ROUTE_TEXT: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText";

	// Substances
	pub const SUBSTANCE_NODE: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:ingredient";
	pub const SUBSTANCE_NAME: &'static str = "hl7:ingredientSubstance/hl7:name";
	pub const SUBSTANCE_TERMID: &'static str =
		"hl7:ingredientSubstance/hl7:code/@code";
	pub const SUBSTANCE_TERMID_VERSION: &'static str =
		"hl7:ingredientSubstance/hl7:code/@codeSystemVersion";
	pub const SUBSTANCE_STRENGTH_VALUE: &'static str =
		"hl7:quantity/hl7:numerator/@value";
	pub const SUBSTANCE_STRENGTH_UNIT: &'static str =
		"hl7:quantity/hl7:numerator/@unit";

	// Dosages
	pub const DOSAGE_NODE: &'static str =
		"hl7:outboundRelationship2[@typeCode='COMP']/hl7:substanceAdministration";
	pub const DOSAGE_TEXT_NODE: &'static str = "hl7:text";
	pub const DOSAGE_FREQUENCY_VALUE: &'static str =
		"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period/@value";
	pub const DOSAGE_FREQUENCY_UNIT: &'static str =
		"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period/@unit";
	pub const DOSAGE_EFFECTIVE_TIME_NULL_FLAVOR: &'static str =
		"hl7:effectiveTime/@nullFlavor";
	pub const DOSAGE_START_DATE: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low/@value";
	pub const DOSAGE_START_DATE_NULL_FLAVOR: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low/@nullFlavor";
	pub const DOSAGE_END_DATE: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high/@value";
	pub const DOSAGE_END_DATE_NULL_FLAVOR: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high/@nullFlavor";
	pub const DOSAGE_DURATION_VALUE: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width/@value";
	pub const DOSAGE_DURATION_UNIT: &'static str =
		"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width/@unit";
	pub const DOSE_VALUE: &'static str = "hl7:doseQuantity/@value";
	pub const DOSE_UNIT: &'static str = "hl7:doseQuantity/@unit";
	pub const ROUTE_CODE: &'static str = "hl7:routeCode/@code";
	pub const ROUTE_CODE_SYSTEM_VERSION: &'static str =
		"hl7:routeCode/@codeSystemVersion";
	pub const DOSE_FORM_TEXT: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode/hl7:originalText";
	pub const DOSE_FORM_TERMID: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode/@code";
	pub const DOSE_FORM_TERMID_VERSION: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode/@codeSystemVersion";
	pub const DOSAGE_BATCH_LOT: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText";
	pub const DOSAGE_PARENT_ROUTE_TERMID: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@code";
	pub const DOSAGE_PARENT_ROUTE_TERMID_VERSION: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@codeSystemVersion";
	pub const DOSAGE_PARENT_ROUTE_TEXT: &'static str =
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText";

	// Indications
	pub const INDICATION_NODE: &'static str =
		"hl7:inboundRelationship[@typeCode='RSON']/hl7:observation/hl7:value";
	pub const INDICATION_TEXT: &'static str = "hl7:originalText";
	pub const INDICATION_CODE: &'static str = "@code";
	pub const INDICATION_VERSION: &'static str = "@codeSystemVersion";

	// Device characteristics
	pub const DEVICE_CHAR_NODE: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic";
	pub const DEVICE_CHAR_CODE: &'static str = "hl7:code/@code";
	pub const DEVICE_CHAR_CODE_SYSTEM: &'static str = "hl7:code/@codeSystem";
	pub const DEVICE_CHAR_DISPLAY: &'static str = "hl7:code/@displayName";
	pub const DEVICE_CHAR_VALUE_TYPE: &'static str = "hl7:value/@xsi:type";
	pub const DEVICE_CHAR_VALUE_TYPE_ALT: &'static str = "hl7:value/@type";
	pub const DEVICE_CHAR_VALUE_VALUE: &'static str = "hl7:value/@value";
	pub const DEVICE_CHAR_VALUE_CODE: &'static str = "hl7:value/@code";
	pub const DEVICE_CHAR_VALUE_CODE_SYSTEM: &'static str = "hl7:value/@codeSystem";
	pub const DEVICE_CHAR_VALUE_DISPLAY: &'static str = "hl7:value/@displayName";
	pub const DEVICE_BRAND_NAME: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[1]";
	pub const COMMON_DEVICE_NAME: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[2]";
	pub const DEVICE_PRODUCT_CODE: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:code/@code";
	pub const DEVICE_MANUFACTURER_NAME: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:name";
	pub const DEVICE_MANUFACTURER_ADDRESS: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:streetAddressLine";
	pub const DEVICE_MANUFACTURER_CITY: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:city";
	pub const DEVICE_MANUFACTURER_STATE: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:state";
	pub const DEVICE_MANUFACTURER_COUNTRY: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:country";
	pub const DEVICE_LOT_NUMBER: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText";
	pub const DEVICE_OPERATOR_CODE: &'static str =
		"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.2.1.1.6']]/hl7:code/@code";
}
