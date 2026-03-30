use super::ValidationProfile;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ValidationRuleMetadata {
	pub code: &'static str,
	pub profile: ValidationProfile,
	pub section: &'static str,
	pub blocking: bool,
	pub message: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleCategory {
	Schema,
	XmlStructure,
	CaseBusiness,
}

impl RuleCategory {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Schema => "schema",
			Self::XmlStructure => "xml_structure",
			Self::CaseBusiness => "case_business",
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationPhase {
	Import,
	CaseValidate,
	Export,
}

impl ValidationPhase {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Import => "import",
			Self::CaseValidate => "case_validate",
			Self::Export => "export",
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
	Blocking,
	Warning,
	Info,
}

impl RuleSeverity {
	pub fn is_blocking(self) -> bool {
		matches!(self, Self::Blocking)
	}

	pub fn as_str(self) -> &'static str {
		match self {
			Self::Blocking => "blocking",
			Self::Warning => "warning",
			Self::Info => "info",
		}
	}
}

pub const VALIDATION_RULES: &[
	ValidationRuleMetadata
] = &[
	ValidationRuleMetadata {
		code: "FDA.C.1.12.RECOMMENDED",
		profile: ValidationProfile::Fda,
		section: "case-identification",
		blocking: false,
		message: "FDA recommends [C.1.12] combination product report indicator.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.12.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "case-identification",
		blocking: true,
		message: "FDA requires [C.1.12] combination product report indicator.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.12.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.C.1.12 combination product report indicator is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "case-identification",
		blocking: true,
		message: "FDA requires [C.1.7.1] when expedited criteria is fulfilled.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.C.1.7.1 local criteria report type is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED.MISSING_CODE",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.1.7.1 local criteria report type missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.2.r.2.EMAIL.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "reporter",
		blocking: true,
		message: "FDA requires reporter email when primary source is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5a.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.5a] IND Number where AE Occurred when C.1.3 is 1/2 and message receiver is CDER_IND/CBER_IND (6 digits).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.FORBIDDEN",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.5.5b must not be provided for postmarket (N.1.4=ZZFDA, N.2.r.3=CDER/CBER).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.5b] Pre-ANDA Number where AE Occurred when C.1.3 is 2 and message receiver is CDER_IND_EXEMPT_BA_BE (6 digits).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.5.5b is required when C.1.3=2 and N.2.r.3=CDER_IND_EXEMPT_BA_BE.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.6.r.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.6.r] cross reported IND when [C.5.5a] is populated.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.11.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "patient",
		blocking: true,
		message: "FDA requires [D.11] patient race when patient payload is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.11.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.D.11 patient race is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.12.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "patient",
		blocking: true,
		message:
			"FDA requires [D.12] patient ethnicity when patient payload is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.12.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.D.12 patient ethnicity is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.E.i.3.2h.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "reactions",
		blocking: true,
		message:
			"FDA requires [E.i.3.2h] when other medically important condition is selected.",
	},
	ValidationRuleMetadata {
		code: "FDA.E.i.3.2h.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.E.i.3.2h required intervention is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.1.A.CONDITIONAL",
		profile: ValidationProfile::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA [G.K.1.A]=1 is allowed only when [C.1.12]=true, [G.K.12.r.1]=true, and [G.k.1]=4 for the same product.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.R.11.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "drugs",
		blocking: false,
		message:
			"FDA recommends [G.K.12.R.11] when [G.K.12.r.1]=true and [C.1.7.1]=4.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.R.3.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA requires [G.K.12.R.3] when [G.K.12.r.1]=true for postmarket ICSRs.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA postmarket requires at least one suspect product with [G.K.12.r.1]=true when [C.1.7.1]=5.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.10a.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.G.k.10a is required when FDA.C.5.5b is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.N.1.4.REQUIRED",
		profile: ValidationProfile::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.N.1.4 batch receiver identifier missing.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.11.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.11.2] Nullification reason is required when [C.1.11.1] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.3.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"C.1.3 must be 2 when premarket receiver and FDA.C.5.5b are present with study type 1/2/3.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.3] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.4.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.4] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.5.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.5] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.6.1.r.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.6.1.r.1] Document description is required when additional documents are available.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.7.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.7] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "C.1.9.1 is true but C.1.9.1.r.1/.r.2 are missing.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.9.1.r.1] Source of the case identifier is required when an other case identifier row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.9.1.r.2] Case identifier is required when an other case identifier row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "Safety report identification is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole/id has extension and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message: "primaryRole/id missing extension; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole/id with root 2.16.840.1.113883.3.989.2.1.3.6 requires extension or nullFlavor.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.2.NAME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole name element has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message: "primaryRole name element is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message:
			"representedOrganization/name has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message:
			"representedOrganization/name is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.4.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reporter",
		blocking: true,
		message: "[C.2.r.4] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.3.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "sender",
		blocking: true,
		message: "[C.3.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.3.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "sender",
		blocking: true,
		message: "[C.3.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.4.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "study",
		blocking: true,
		message:
			"[C.5.4] Study type where reaction(s) / event(s) were observed is required when [C.1.3] is report from study (2).",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "study",
		blocking: true,
		message:
			"researchStudy/title has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "study",
		blocking: true,
		message: "researchStudy/title is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message: "[D.1] This Element is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.2.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.2.2a] Parent age is required when [D.10.2.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.2.2b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.2.2b] Parent age unit is required when [D.10.2.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.6.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message: "[D.10.6] Parent sex is required when parent data is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.7.1.r.1a] Parent medical history MedDRA version is required when [D.10.7.1.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.7.1.r.1b] Parent medical history MedDRA code is required when [D.10.7.1.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.MPID_PHPID.EXCLUSIVE",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.2b/D.10.8.r.3b] Any given parent past drug entry may have either MPID or PhPID, but not both.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.2a] Parent past drug MPID version is required when [D.10.8.r.2b] MPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.3a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.3a] Parent past drug PhPID version is required when [D.10.8.r.3b] PhPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.6a] Parent past drug indication MedDRA version is required when [D.10.8.r.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.6b] Parent past drug indication MedDRA code is required when [D.10.8.r.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.7a] Parent past drug reaction MedDRA version is required when [D.10.8.r.7b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.7b] Parent past drug reaction MedDRA code is required when [D.10.8.r.7a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2.1a] Gestation period is required when [D.2.2.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2.1b] Gestation period unit is required when [D.2.2.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2a] Age at time of onset is required when [D.2.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2b] Age unit is required when [D.2.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.BIRTHTIME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"birthTime has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message: "birthTime missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.5.SEX.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message: "administrativeGenderCode missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.7.1.r.1a] MedDRA version for medical history is required when [D.7.1.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.7.1.r.1b] Medical history MedDRA code is required when [D.7.1.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.2.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "D.7.2 must be provided when D.7.1.r.1b is not provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.MPID_PHPID.EXCLUSIVE",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.2b/D.8.r.3b] Any given past drug entry may have either MPID or PhPID, but not both.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.2a] Past drug MPID version is required when [D.8.r.2b] MPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.3a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.3a] Past drug PhPID version is required when [D.8.r.3b] PhPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.6a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.6a] Indication MedDRA version is required when [D.8.r.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.6b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.6b] Indication MedDRA code is required when [D.8.r.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.7a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.7a] Reaction MedDRA version is required when [D.8.r.7b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.7b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.7b] Reaction MedDRA code is required when [D.8.r.7a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.1a] Reported cause of death MedDRA version is required when [D.9.2.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.1b] Reported cause of death MedDRA code is required when [D.9.2.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.2] Reported cause of death comments are required when [D.9.2.r.1] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.3] Autopsy was performed is required when [D.9.1] date of death is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.1a] Autopsy cause of death MedDRA version is required when [D.9.4.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.1b] Autopsy cause of death MedDRA code is required when [D.9.4.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.2] Autopsy cause of death comments are required when [D.9.4.r.1] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"patient effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson birthTime has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson birthTime missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson name element has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson name element is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"relatedInvestigation/code has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"relatedInvestigation/code missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.1.1a] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.1.1b] is required when [E.i.1.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.2.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"reaction translation has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.2.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "reaction translation missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.2.1a] Reaction MedDRA version is required when [E.i.2.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.2.1b] Reaction MedDRA code is required when a reaction row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"reaction term has code and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "reaction term missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"reaction effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.4-6.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: false,
		message: "Reaction should include start, end, or duration.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.6a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.6a] Reaction duration is required when [E.i.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.6b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.6b] Reaction duration unit is required when [E.i.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.7.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message:
			"reaction outcome value has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.7.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "reaction outcome value missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.7.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.7] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.9.COUNTRY.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "reactions",
		blocking: true,
		message: "reaction country missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message: "[F.r.1] Test date is required when [F.r.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.1] Test name (free text) is required when [F.r.1] is populated and [F.r.2.2b] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.2a] Test name MedDRA version is required when [F.r.2.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.2b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.2b] Test name MedDRA code is required when [F.r.1] is populated and [F.r.2.1] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message: "[F.r.2] is required when test payload is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.1] Test result (coded) is required when [F.r.2] is populated and neither [F.r.3.2] nor [F.r.3.4] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.2] Test result (value/finding) is required when [F.r.2] is populated and [F.r.3.1] and [F.r.3.4] are not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.3] Test result unit is required when [F.r.3.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.4.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.4] Result unstructured data is required when [F.r.2] is populated and [F.r.3] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message: "[G.k.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.1.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.1.1a] MPID version is required when [G.k.2.1.1b] MPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.1.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.1.2a] PhPID version is required when [G.k.2.1.2b] PhPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message: "[G.k.2.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.NAME.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"ingredientSubstance/name has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.NAME.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message: "ingredientSubstance/name is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.1] Substance name is required when an active substance row has no TermID.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.2a] Substance TermID version is required when [G.k.2.3.r.2b] TermID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.3b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.3b] Strength unit is required when [G.k.2.3.r.3a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.10.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.10.2a] Route of administration TermID version is required when [G.k.4.r.10.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"formCode missing code/codeSystem/originalText; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.11.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.11.2a] Parent route TermID version is required when [G.k.4.r.11.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message: "routeCode missing code; originalText or nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.1b] Dose unit is required when [G.k.4.r.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.3] Time interval unit is required when [G.k.4.r.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"drug effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.4-8.CONDITIONAL",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message: "Drug requires start, end, or duration.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.6a] Duration value is required when [G.k.4.r.6b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.6b] Duration unit is required when [G.k.4.r.6a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.9.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.9.2a] Dose form TermID version is required when [G.k.4.r.9.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.5a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.5a] Cumulative dose to first reaction value is required when [G.k.5b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.5b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.5b] Cumulative dose to first reaction unit is required when [G.k.5a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.6a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.6a] Gestation period at exposure value is required when [G.k.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.6b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.6b] Gestation period at exposure unit is required when [G.k.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.7.r.2a] Indication MedDRA version is required when [G.k.7.r.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.7.r.2b] Indication MedDRA code is required when [G.k.7.r.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.2.ID.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"adverseEventAssessment/id has extension and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.2.ID.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"adverseEventAssessment/id missing extension; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.1a] Administration start interval value is required when [G.k.9.i.3.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.1b] Administration start interval unit is required when [G.k.9.i.3.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.2a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.2a] Last-dose interval value is required when [G.k.9.i.3.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.2b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.2b] Last-dose interval unit is required when [G.k.9.i.3.2a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.1.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "narrative",
		blocking: true,
		message: "[H.1] This Element is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.3.r.1a.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.3.r.1a] Sender diagnosis MedDRA version is required when [H.3.r.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.3.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.3.r.1b] Sender diagnosis MedDRA code is required when [H.3.r.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.5.r.1b.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.5.r.1b] Case summary language is required when [H.5.r.1a] summary type is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.2] Batch number is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.3] Batch sender identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.4.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.4] Batch receiver identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.5.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.5] Date of batch transmission is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.2.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.2] Message sender identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.3.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.3] Message receiver identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "case-identification",
		blocking: true,
		message: "Message header is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.BL.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"BL value has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.BL.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "BL value missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.CODE.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"code has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.CODE.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"code missing code/codeSystem; nullFlavor is required when originalText is absent.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "ISO country code must be 2 uppercase letters.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message:
			"Document text compression attribute must be removed for export compatibility.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "doseQuantity must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.EFFECTIVETIME.WIDTH.REQUIRES_BOUND",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "effectiveTime with width must include low/high.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.GK11.EMPTY.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message: "Empty G.k.11 relationships should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"investigationCharacteristic BL has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"investigationCharacteristic BL missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.IVL_TS.OPERATOR_A.BOUND_REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_TS operator='A' must include low, high, or width.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.LOW_HIGH.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"low/high has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "MedDRA code must be 8 digits.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.MEDDRA.VERSION.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "MedDRA code requires codeSystemVersion.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.OPTIONAL.PATH.EMPTY.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message:
			"Optional-path nodes without real data should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PERIOD.VALUE_UNIT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "period must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PIVL_TS.PERIOD.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "PIVL_TS must include period.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "PIVL_TS period must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message: "Known placeholder codeSystemVersion attributes should be removed during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "Placeholder values are not allowed in XML content or attributes.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PLACEHOLDER.VALUE.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message: "Known placeholder value nodes should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.RACE.EMPTY.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message: "Empty race nodes should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.RACE.NI.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message: "Race NI placeholder nodes should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.ROOT.ITSVERSION.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "Root ITSVersion must be present and set to XML_1.0.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"Root xsi:schemaLocation must be present and reference the expected root schema.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.STRUCTURAL.EMPTY.PRUNE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message:
			"Empty structural wrapper nodes should be pruned during export.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message:
			"Summary language=JA attribute must be removed for export compatibility.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.SXPR_TS.COMP.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "SXPR_TS must include at least one comp element.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.FORMAT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"telecom value must start with tel:, fax:, or mailto:.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"telecom has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "telecom missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_PQ must include low/high/center.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_PQ low/high/center must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "PQ must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "Unsupported test result xsi:type.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TEXT.NULLFLAVOR.FORBIDDEN",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message:
			"text/originalText has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TEXT.NULLFLAVOR.REQUIRED",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: true,
		message: "text/originalText is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.XSI_TYPE.NORMALIZE",
		profile: ValidationProfile::Ich,
		section: "xml",
		blocking: false,
		message:
			"Promote non-namespaced type attribute to xsi:type for export compatibility.",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.2.r.4.KR.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "reporter",
		blocking: true,
		message:
			"MFDS requires [C.2.r.4.KR.1] when reporter qualification [C.2.r.4] is other health professional (3).",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "case-identification",
		blocking: true,
		message: "MFDS KR profile does not allow sender type 3.",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "study",
		blocking: true,
		message:
			"MFDS requires [C.5.4.KR.1] when study type [C.5.4] is other studies (3).",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires parent past drug code version [D.10.8.r.1.KR.1a] for FR when [D.10.8.r.1.KR.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires parent past drug code [D.10.8.r.1.KR.1b] for KR/FR receiver profiles.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1a.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires past drug code version [D.8.r.1.KR.1a] for FR when [D.8.r.1.KR.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1b.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires past drug code [D.8.r.1.KR.1b] for KR/FR receiver profiles.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1a.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires product code version [G.k.2.1.KR.1a] for FR when product code is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1b.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires product code [G.k.2.1.KR.1b] for KR/FR receiver profiles.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires substance code version [G.k.2.3.r.1.KR.1a] for FR when substance code is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires substance code [G.k.2.3.r.1.KR.1b] for KR/FR when product code is not provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires source of assessment when KR method/result values are provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires KR method of assessment when source of assessment is present.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires WHO-UMC result when source is present and method is WHO-UMC (1).",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires KRCT result when source is present, method is KRCT (2), and report is clinical (CT/CU).",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS domestic cases should provide KR ingredient coding for each active substance.",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: true,
		message: "MFDS domestic cases require KR product coding for the drug.",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED",
		profile: ValidationProfile::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS foreign-use products should provide WHO MPID/KR product coding.",
	},
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportDirective {
	OutcomeDefaultCode3,
	RequiredInterventionNullFlavorNi,
	DrugRoleDefaultConcomitant,
	ClearNullFlavorWhenValuePresent,
	NormalizeInvalidCodeToNullFlavorNi,
	NormalizeTypeAttributeToXsiType,
	RemoveDocumentTextCompression,
	RemoveSummaryLanguageJa,
	RemovePlaceholderValueNodes,
	RemovePlaceholderCodeSystemVersion,
	RemoveRaceNiNodes,
	RemoveRaceEmptyNodes,
	RemoveEmptyGk11Relationships,
	RemoveOptionalPathEmptyNodes,
	RemoveEmptyStructuralNodes,
}

impl ExportDirective {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::OutcomeDefaultCode3 => "outcome_default_code_3",
			Self::RequiredInterventionNullFlavorNi => {
				"required_intervention_null_flavor_ni"
			}
			Self::DrugRoleDefaultConcomitant => "drug_role_default_concomitant",
			Self::ClearNullFlavorWhenValuePresent => {
				"clear_null_flavor_when_value_present"
			}
			Self::NormalizeInvalidCodeToNullFlavorNi => {
				"normalize_invalid_code_to_null_flavor_ni"
			}
			Self::NormalizeTypeAttributeToXsiType => {
				"normalize_type_attribute_to_xsi_type"
			}
			Self::RemoveDocumentTextCompression => {
				"remove_document_text_compression"
			}
			Self::RemoveSummaryLanguageJa => "remove_summary_language_ja",
			Self::RemovePlaceholderValueNodes => "remove_placeholder_value_nodes",
			Self::RemovePlaceholderCodeSystemVersion => {
				"remove_placeholder_code_system_version"
			}
			Self::RemoveRaceNiNodes => "remove_race_ni_nodes",
			Self::RemoveRaceEmptyNodes => "remove_race_empty_nodes",
			Self::RemoveEmptyGk11Relationships => "remove_empty_gk11_relationships",
			Self::RemoveOptionalPathEmptyNodes => "remove_optional_path_empty_nodes",
			Self::RemoveEmptyStructuralNodes => "remove_empty_structural_nodes",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportNormalizeKind {
	AsciiDigitsLen(usize),
	AsciiUpperLen(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportNormalizationSpec {
	pub xpath: &'static str,
	pub attribute: &'static str,
	pub kind: ExportNormalizeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportAttributeStripSpec {
	pub xpath: &'static str,
	pub attribute: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCondition {
	Always,
	IchCaseHistoryTrueMissingPriorIds,
	IchMedicalHistoryMissingD72Text,
	IchReportTypeIsStudy,
	FdaFulfilExpeditedCriteriaTrue,
	FdaReactionOtherMedicallyImportantTrue,
	FdaPrimarySourcePresent,
	FdaPatientPayloadPresent,
	FdaIndNumberRequired,
	FdaPreAndaNumberRequired,
	FdaCrossReportedIndRequired,
	FdaPreAndaRequired,
	FdaPreAndaForbidden,
	FdaGk10aRequired,
	FdaPremarketReportTypeMustBeTwo,
	MfdsRelatednessSourcePresent,
	MfdsRelatednessMethodRequiredContext,
	MfdsRelatednessKr1RequiredContext,
	MfdsRelatednessKr2RequiredContext,
	MfdsRelatednessMethodOrResultPresent,
	MfdsProductCodeRequiredContext,
	MfdsProductVersionRequiredContext,
	MfdsSubstanceCodeRequiredContext,
	MfdsSubstanceVersionRequiredContext,
	MfdsPastDrugCodeRequiredContext,
	MfdsPastDrugVersionRequiredContext,
	MfdsParentPastDrugCodeRequiredContext,
	MfdsParentPastDrugVersionRequiredContext,
	MfdsDrugDomesticKr,
	MfdsDrugForeignNonKr,
	MfdsSenderTypeDisallowed,
	MfdsPrimarySourceQualificationIsThree,
	MfdsStudyTypeReactionIsThree,
	IchTestPayloadPresent,
}

impl RuleCondition {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Always => "always",
			Self::IchCaseHistoryTrueMissingPriorIds => {
				"ich_case_history_true_missing_prior_ids"
			}
			Self::IchMedicalHistoryMissingD72Text => {
				"ich_medical_history_missing_d72_text"
			}
			Self::IchReportTypeIsStudy => "ich_report_type_is_study",
			Self::FdaFulfilExpeditedCriteriaTrue => {
				"fda_fulfil_expedited_criteria_true"
			}
			Self::FdaReactionOtherMedicallyImportantTrue => {
				"fda_reaction_other_medically_important_true"
			}
			Self::FdaPrimarySourcePresent => "fda_primary_source_present",
			Self::FdaPatientPayloadPresent => "fda_patient_payload_present",
			Self::FdaIndNumberRequired => "fda_ind_number_required",
			Self::FdaPreAndaNumberRequired => "fda_pre_anda_number_required",
			Self::FdaCrossReportedIndRequired => "fda_cross_reported_ind_required",
			Self::FdaPreAndaRequired => "fda_pre_anda_required",
			Self::FdaPreAndaForbidden => "fda_pre_anda_forbidden",
			Self::FdaGk10aRequired => "fda_g_k_10a_required",
			Self::FdaPremarketReportTypeMustBeTwo => {
				"fda_premarket_report_type_must_be_two"
			}
			Self::MfdsRelatednessSourcePresent => "mfds_relatedness_source_present",
			Self::MfdsRelatednessMethodRequiredContext => {
				"mfds_relatedness_method_required_context"
			}
			Self::MfdsRelatednessKr1RequiredContext => {
				"mfds_relatedness_kr1_required_context"
			}
			Self::MfdsRelatednessKr2RequiredContext => {
				"mfds_relatedness_kr2_required_context"
			}
			Self::MfdsRelatednessMethodOrResultPresent => {
				"mfds_relatedness_method_or_result_present"
			}
			Self::MfdsProductCodeRequiredContext => {
				"mfds_product_code_required_context"
			}
			Self::MfdsProductVersionRequiredContext => {
				"mfds_product_version_required_context"
			}
			Self::MfdsSubstanceCodeRequiredContext => {
				"mfds_substance_code_required_context"
			}
			Self::MfdsSubstanceVersionRequiredContext => {
				"mfds_substance_version_required_context"
			}
			Self::MfdsPastDrugCodeRequiredContext => {
				"mfds_past_drug_code_required_context"
			}
			Self::MfdsPastDrugVersionRequiredContext => {
				"mfds_past_drug_version_required_context"
			}
			Self::MfdsParentPastDrugCodeRequiredContext => {
				"mfds_parent_past_drug_code_required_context"
			}
			Self::MfdsParentPastDrugVersionRequiredContext => {
				"mfds_parent_past_drug_version_required_context"
			}
			Self::MfdsDrugDomesticKr => "mfds_drug_domestic_kr",
			Self::MfdsDrugForeignNonKr => "mfds_drug_foreign_non_kr",
			Self::MfdsSenderTypeDisallowed => "mfds_sender_type_disallowed",
			Self::MfdsPrimarySourceQualificationIsThree => {
				"mfds_primary_source_qualification_is_three"
			}
			Self::MfdsStudyTypeReactionIsThree => {
				"mfds_study_type_reaction_is_three"
			}
			Self::IchTestPayloadPresent => "ich_test_payload_present",
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleFacts {
	pub ich_case_history_true_missing_prior_ids: Option<bool>,
	pub ich_medical_history_missing_d72_text: Option<bool>,
	pub ich_report_type_is_study: Option<bool>,
	pub fda_fulfil_expedited_criteria: Option<bool>,
	pub fda_reaction_other_medically_important: Option<bool>,
	pub fda_combination_product_true: Option<bool>,
	pub fda_primary_source_present: Option<bool>,
	pub fda_patient_payload_present: Option<bool>,
	pub fda_type_of_report_is_one_or_two: Option<bool>,
	pub fda_type_of_report_is_two: Option<bool>,
	pub fda_msg_receiver_is_cder_ind_or_cber_ind: Option<bool>,
	pub fda_msg_receiver_is_cder_ind_exempt_ba_be: Option<bool>,
	pub fda_has_ind_number: Option<bool>,
	pub fda_has_pre_anda: Option<bool>,
	pub fda_batch_receiver_is_zzfda: Option<bool>,
	pub fda_msg_receiver_is_cder_or_cber: Option<bool>,
	pub fda_batch_receiver_is_zzfda_premarket: Option<bool>,
	pub fda_msg_receiver_is_premarket: Option<bool>,
	pub fda_study_type_is_1_2_3: Option<bool>,
	pub mfds_relatedness_source_present: Option<bool>,
	pub mfds_relatedness_method_required_context: Option<bool>,
	pub mfds_relatedness_kr1_required_context: Option<bool>,
	pub mfds_relatedness_kr2_required_context: Option<bool>,
	pub mfds_relatedness_method_present: Option<bool>,
	pub mfds_relatedness_result_present: Option<bool>,
	pub mfds_product_code_required_context: Option<bool>,
	pub mfds_product_version_required_context: Option<bool>,
	pub mfds_substance_code_required_context: Option<bool>,
	pub mfds_substance_version_required_context: Option<bool>,
	pub mfds_past_drug_code_required_context: Option<bool>,
	pub mfds_past_drug_version_required_context: Option<bool>,
	pub mfds_parent_past_drug_code_required_context: Option<bool>,
	pub mfds_parent_past_drug_version_required_context: Option<bool>,
	pub mfds_drug_domestic_kr: Option<bool>,
	pub mfds_drug_foreign_non_kr: Option<bool>,
	pub mfds_sender_type_disallowed: Option<bool>,
	pub mfds_primary_source_qualification_is_three: Option<bool>,
	pub mfds_study_type_reaction_is_three: Option<bool>,
	pub ich_test_payload_present: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalRule<'a> {
	pub code: &'a str,
	pub profile: ValidationProfile,
	pub section: &'a str,
	pub blocking: bool,
	pub category: RuleCategory,
	pub phases: &'a [ValidationPhase],
	pub severity: RuleSeverity,
	pub message: &'a str,
	pub condition: RuleCondition,
	pub export_directive: Option<ExportDirective>,
}

const PHASES_IMPORT_AND_EXPORT: &[ValidationPhase] =
	&[ValidationPhase::Import, ValidationPhase::Export];
const PHASES_CASE_VALIDATE_AND_EXPORT: &[ValidationPhase] =
	&[ValidationPhase::CaseValidate, ValidationPhase::Export];
const PHASES_CASE_VALIDATE: &[ValidationPhase] = &[ValidationPhase::CaseValidate];
const PHASES_EXPORT_ONLY: &[ValidationPhase] = &[ValidationPhase::Export];
const PHASES_IMPORT_ONLY: &[ValidationPhase] = &[ValidationPhase::Import];

#[derive(Debug, Clone, Copy)]
struct ConditionBinding {
	code: &'static str,
	condition: RuleCondition,
}

const CONDITION_BINDINGS: &[ConditionBinding] = &[
	ConditionBinding {
		code: "FDA.C.1.7.1.REQUIRED",
		condition: RuleCondition::FdaFulfilExpeditedCriteriaTrue,
	},
	ConditionBinding {
		code: "FDA.C.2.r.2.EMAIL.REQUIRED",
		condition: RuleCondition::FdaPrimarySourcePresent,
	},
	ConditionBinding {
		code: "FDA.C.5.5a.REQUIRED",
		condition: RuleCondition::FdaIndNumberRequired,
	},
	ConditionBinding {
		code: "FDA.C.5.5b.FORBIDDEN",
		condition: RuleCondition::FdaPreAndaForbidden,
	},
	ConditionBinding {
		code: "FDA.C.5.5b.REQUIRED",
		condition: RuleCondition::FdaPreAndaRequired,
	},
	ConditionBinding {
		code: "FDA.C.5.6.r.REQUIRED",
		condition: RuleCondition::FdaCrossReportedIndRequired,
	},
	ConditionBinding {
		code: "FDA.D.11.REQUIRED",
		condition: RuleCondition::FdaPatientPayloadPresent,
	},
	ConditionBinding {
		code: "FDA.D.12.REQUIRED",
		condition: RuleCondition::FdaPatientPayloadPresent,
	},
	ConditionBinding {
		code: "FDA.E.i.3.2h.REQUIRED",
		condition: RuleCondition::FdaReactionOtherMedicallyImportantTrue,
	},
	ConditionBinding {
		code: "FDA.G.k.10a.REQUIRED",
		condition: RuleCondition::FdaGk10aRequired,
	},
	ConditionBinding {
		code: "ICH.C.1.3.CONDITIONAL",
		condition: RuleCondition::FdaPremarketReportTypeMustBeTwo,
	},
	ConditionBinding {
		code: "ICH.C.1.9.1.CONDITIONAL",
		condition: RuleCondition::IchCaseHistoryTrueMissingPriorIds,
	},
	ConditionBinding {
		code: "ICH.C.5.4.REQUIRED",
		condition: RuleCondition::IchReportTypeIsStudy,
	},
	ConditionBinding {
		code: "ICH.D.7.2.CONDITIONAL",
		condition: RuleCondition::IchMedicalHistoryMissingD72Text,
	},
	ConditionBinding {
		code: "ICH.F.r.2.REQUIRED",
		condition: RuleCondition::IchTestPayloadPresent,
	},
	ConditionBinding {
		code: "MFDS.C.2.r.4.KR.1.REQUIRED",
		condition: RuleCondition::MfdsPrimarySourceQualificationIsThree,
	},
	ConditionBinding {
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		condition: RuleCondition::MfdsSenderTypeDisallowed,
	},
	ConditionBinding {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		condition: RuleCondition::MfdsStudyTypeReactionIsThree,
	},
	ConditionBinding {
		code: "MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		condition: RuleCondition::MfdsParentPastDrugVersionRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		condition: RuleCondition::MfdsParentPastDrugCodeRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.D.8.r.1.KR.1a.REQUIRED",
		condition: RuleCondition::MfdsPastDrugVersionRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.D.8.r.1.KR.1b.REQUIRED",
		condition: RuleCondition::MfdsPastDrugCodeRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.2.1.KR.1a.REQUIRED",
		condition: RuleCondition::MfdsProductVersionRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.2.1.KR.1b.REQUIRED",
		condition: RuleCondition::MfdsProductCodeRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		condition: RuleCondition::MfdsSubstanceVersionRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		condition: RuleCondition::MfdsSubstanceCodeRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.9.i.2.r.1.REQUIRED",
		condition: RuleCondition::MfdsRelatednessMethodOrResultPresent,
	},
	ConditionBinding {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		condition: RuleCondition::MfdsRelatednessMethodRequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		condition: RuleCondition::MfdsRelatednessKr1RequiredContext,
	},
	ConditionBinding {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		condition: RuleCondition::MfdsRelatednessKr2RequiredContext,
	},
	ConditionBinding {
		code: "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		condition: RuleCondition::MfdsDrugDomesticKr,
	},
	ConditionBinding {
		code: "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		condition: RuleCondition::MfdsDrugDomesticKr,
	},
	ConditionBinding {
		code: "MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED",
		condition: RuleCondition::MfdsDrugForeignNonKr,
	},
];

#[derive(Debug, Clone, Copy)]
enum ValuePolicy {
	NonEmpty,
	NonEmptyOrNullFlavor,
	SixDigitsNumeric,
	FdaRaceCodeOrNullFlavor,
	FdaEthnicityCodeOrNullFlavor,
	FdaGk10aCodeOrNa,
	FdaLocalCriteriaCodeByFacts,
	IchC13ConditionalMustBeTwo,
}

#[derive(Debug, Clone, Copy)]
struct ValuePolicyBinding {
	code: &'static str,
	policy: ValuePolicy,
}

const VALUE_POLICY_BINDINGS: &[ValuePolicyBinding] = &[
	ValuePolicyBinding {
		code: "FDA.C.1.12.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.C.1.7.1.REQUIRED",
		policy: ValuePolicy::FdaLocalCriteriaCodeByFacts,
	},
	ValuePolicyBinding {
		code: "FDA.C.1.7.1.REQUIRED.MISSING_CODE",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.C.5.5a.REQUIRED",
		policy: ValuePolicy::SixDigitsNumeric,
	},
	ValuePolicyBinding {
		code: "FDA.C.5.5b.REQUIRED",
		policy: ValuePolicy::SixDigitsNumeric,
	},
	ValuePolicyBinding {
		code: "FDA.D.11.REQUIRED",
		policy: ValuePolicy::FdaRaceCodeOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.D.12.REQUIRED",
		policy: ValuePolicy::FdaEthnicityCodeOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.E.i.3.2h.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.G.k.10a.REQUIRED",
		policy: ValuePolicy::FdaGk10aCodeOrNa,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.2.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.3.CONDITIONAL",
		policy: ValuePolicy::IchC13ConditionalMustBeTwo,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.4.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.5.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.6.1.r.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.7.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.2.r.4.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.3.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.3.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.C.5.4.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.2.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.2.2b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.6.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.3a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.3a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.6a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.6b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.7a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.7b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.2.r.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.9.4.r.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.1.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.1.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.2.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.2.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.7.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.2.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.2.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.2.2b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.3.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.1.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.1.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.3.r.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.3.r.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.2.3.r.3b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.10.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.11.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.4.r.9.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.5a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.5b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.6a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.6b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.9.i.3.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.9.i.3.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.9.i.3.2a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.9.i.3.2b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.H.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.H.3.r.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.H.3.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.H.5.r.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.1.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.1.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.1.4.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.1.5.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.2.r.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.N.2.r.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.C.2.r.4.KR.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.D.8.r.1.KR.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.D.8.r.1.KR.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.2.1.KR.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.2.1.KR.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.9.i.2.r.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED",
		policy: ValuePolicy::NonEmpty,
	},
];

#[derive(Debug, Clone, Copy)]
struct PresencePolicyBinding {
	code: &'static str,
}

const REQUIRED_PRESENCE_BINDINGS: &[PresencePolicyBinding] = &[
	PresencePolicyBinding {
		code: "FDA.C.2.r.2.EMAIL.REQUIRED",
	},
	PresencePolicyBinding {
		code: "FDA.N.1.4.REQUIRED",
	},
];

fn export_directive_for_code(code: &str) -> Option<ExportDirective> {
	match code {
		"FDA.E.i.3.2h.REQUIRED" => {
			Some(ExportDirective::RequiredInterventionNullFlavorNi)
		}
		"FDA.C.1.7.1.REQUIRED" | "FDA.C.1.12.REQUIRED" => {
			Some(ExportDirective::ClearNullFlavorWhenValuePresent)
		}
		"ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED"
		| "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED" => {
			Some(ExportDirective::NormalizeInvalidCodeToNullFlavorNi)
		}
		"ICH.XML.XSI_TYPE.NORMALIZE" => {
			Some(ExportDirective::NormalizeTypeAttributeToXsiType)
		}
		"ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN" => {
			Some(ExportDirective::RemoveDocumentTextCompression)
		}
		"ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN" => {
			Some(ExportDirective::RemoveSummaryLanguageJa)
		}
		"ICH.XML.PLACEHOLDER.VALUE.PRUNE" => {
			Some(ExportDirective::RemovePlaceholderValueNodes)
		}
		"ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE" => {
			Some(ExportDirective::RemovePlaceholderCodeSystemVersion)
		}
		"ICH.XML.RACE.NI.PRUNE" => Some(ExportDirective::RemoveRaceNiNodes),
		"ICH.XML.RACE.EMPTY.PRUNE" => Some(ExportDirective::RemoveRaceEmptyNodes),
		"ICH.XML.GK11.EMPTY.PRUNE" => {
			Some(ExportDirective::RemoveEmptyGk11Relationships)
		}
		"ICH.XML.OPTIONAL.PATH.EMPTY.PRUNE" => {
			Some(ExportDirective::RemoveOptionalPathEmptyNodes)
		}
		"ICH.XML.STRUCTURAL.EMPTY.PRUNE" => {
			Some(ExportDirective::RemoveEmptyStructuralNodes)
		}
		_ => None,
	}
}

fn condition_for_code(code: &str) -> RuleCondition {
	CONDITION_BINDINGS
		.iter()
		.find(|binding| binding.code == code)
		.map(|binding| binding.condition)
		.unwrap_or(RuleCondition::Always)
}

fn to_canonical_rule<'a>(rule: &'a ValidationRuleMetadata) -> CanonicalRule<'a> {
	let export_directive = export_directive_for_code(rule.code);
	let category = category_for_rule(rule);
	let phases = phases_for_rule(rule, export_directive);
	let severity = severity_for_rule(rule);
	CanonicalRule {
		code: rule.code,
		profile: rule.profile,
		section: rule.section,
		blocking: severity.is_blocking(),
		category,
		phases,
		severity,
		message: rule.message,
		condition: condition_for_code(rule.code),
		export_directive,
	}
}

fn category_for_rule(rule: &ValidationRuleMetadata) -> RuleCategory {
	if is_xml_structure_rule(rule) {
		RuleCategory::XmlStructure
	} else {
		RuleCategory::CaseBusiness
	}
}

fn phases_for_rule(
	rule: &ValidationRuleMetadata,
	export_directive: Option<ExportDirective>,
) -> &'static [ValidationPhase] {
	if export_directive.is_some() {
		if is_export_only_rule(rule.code) {
			return PHASES_EXPORT_ONLY;
		}
		if is_xml_structure_rule(rule) {
			return PHASES_IMPORT_AND_EXPORT;
		}
		return PHASES_CASE_VALIDATE_AND_EXPORT;
	}
	if is_xml_structure_rule(rule) {
		return PHASES_IMPORT_ONLY;
	}
	PHASES_CASE_VALIDATE
}

fn is_export_only_rule(code: &str) -> bool {
	code.contains(".PRUNE")
		|| code.contains(".NORMALIZE")
		|| matches!(
			code,
			"ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN"
				| "ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN"
		)
}

fn is_xml_structure_rule(rule: &ValidationRuleMetadata) -> bool {
	if rule.section == "xml" {
		return true;
	}
	if rule.code.contains(".NULLFLAVOR.") {
		return true;
	}
	matches!(
		rule.code,
		"ICH.C.1.3.CONDITIONAL"
			| "ICH.C.1.9.1.CONDITIONAL"
			| "ICH.D.7.2.CONDITIONAL"
			| "ICH.D.5.SEX.CONDITIONAL"
			| "ICH.E.i.4-6.CONDITIONAL"
			| "ICH.G.k.4.r.4-8.CONDITIONAL"
	)
}

fn severity_for_rule(rule: &ValidationRuleMetadata) -> RuleSeverity {
	if rule.code.ends_with(".RECOMMENDED")
		|| rule.code.contains(".PRUNE")
		|| rule.code.contains(".NORMALIZE")
	{
		return RuleSeverity::Info;
	}
	if rule.blocking {
		RuleSeverity::Blocking
	} else {
		RuleSeverity::Warning
	}
}

fn rule_applies_in_phase(rule: CanonicalRule<'_>, phase: ValidationPhase) -> bool {
	rule.phases.contains(&phase)
}

pub fn find_canonical_rule_for_phase(
	code: &str,
	phase: ValidationPhase,
) -> Option<CanonicalRule<'static>> {
	VALIDATION_RULES
		.iter()
		.filter(|rule| rule.code == code)
		.map(to_canonical_rule)
		.find(|rule| rule_applies_in_phase(*rule, phase))
}

pub fn find_canonical_rule(code: &str) -> Option<CanonicalRule<'static>> {
	find_canonical_rule_for_phase(code, ValidationPhase::CaseValidate).or_else(
		|| {
			VALIDATION_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_rule)
		},
	)
}

pub fn canonical_rules_for_profile(
	profile: ValidationProfile,
) -> Vec<CanonicalRule<'static>> {
	VALIDATION_RULES
		.iter()
		.filter(|rule| {
			matches!(rule.profile, ValidationProfile::Ich) || rule.profile == profile
		})
		.map(to_canonical_rule)
		.collect()
}

pub fn canonical_rules_for_profile_phase(
	profile: ValidationProfile,
	phase: ValidationPhase,
) -> Vec<CanonicalRule<'static>> {
	canonical_rules_for_profile(profile)
		.into_iter()
		.filter(|rule| rule_applies_in_phase(*rule, phase))
		.collect()
}

pub fn canonical_rules_all() -> Vec<CanonicalRule<'static>> {
	VALIDATION_RULES.iter().map(to_canonical_rule).collect()
}

pub fn canonical_rules_for_phase(
	phase: ValidationPhase,
) -> Vec<CanonicalRule<'static>> {
	canonical_rules_all()
		.into_iter()
		.filter(|rule| rule_applies_in_phase(*rule, phase))
		.collect()
}

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
	const FNV_PRIME: u64 = 1099511628211;
	for b in bytes {
		hash ^= *b as u64;
		hash = hash.wrapping_mul(FNV_PRIME);
	}
	hash
}

pub fn canonical_rules_version(profile: Option<ValidationProfile>) -> String {
	let rules = if let Some(profile) = profile {
		canonical_rules_for_profile(profile)
	} else {
		canonical_rules_all()
	};

	let mut hash: u64 = 14695981039346656037;
	for rule in rules {
		hash = fnv1a_update(hash, rule.code.as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.profile.as_str().as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.section.as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.severity.as_str().as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.category.as_str().as_bytes());
		hash = fnv1a_update(hash, b"|");
		for phase in rule.phases {
			hash = fnv1a_update(hash, phase.as_str().as_bytes());
			hash = fnv1a_update(hash, b",");
		}
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.message.as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.condition.as_str().as_bytes());
		hash = fnv1a_update(hash, b"|");
		if let Some(d) = rule.export_directive {
			hash = fnv1a_update(hash, d.as_str().as_bytes());
		}
		hash = fnv1a_update(hash, b";");
	}

	format!("{hash:016x}")
}

pub fn is_rule_condition_satisfied(code: &str, facts: RuleFacts) -> bool {
	let Some(rule) = find_canonical_rule(code) else {
		return true;
	};
	match rule.condition {
		RuleCondition::Always => true,
		RuleCondition::IchCaseHistoryTrueMissingPriorIds => facts
			.ich_case_history_true_missing_prior_ids
			.unwrap_or(false),
		RuleCondition::IchMedicalHistoryMissingD72Text => {
			facts.ich_medical_history_missing_d72_text.unwrap_or(false)
		}
		RuleCondition::IchReportTypeIsStudy => {
			facts.ich_report_type_is_study.unwrap_or(false)
		}
		RuleCondition::FdaFulfilExpeditedCriteriaTrue => {
			facts.fda_fulfil_expedited_criteria.unwrap_or(false)
		}
		RuleCondition::FdaReactionOtherMedicallyImportantTrue => facts
			.fda_reaction_other_medically_important
			.unwrap_or(false),
		RuleCondition::FdaPrimarySourcePresent => {
			facts.fda_primary_source_present.unwrap_or(false)
		}
		RuleCondition::FdaPatientPayloadPresent => {
			facts.fda_patient_payload_present.unwrap_or(false)
		}
		RuleCondition::FdaIndNumberRequired => {
			facts.fda_type_of_report_is_one_or_two.unwrap_or(false)
				&& facts
					.fda_msg_receiver_is_cder_ind_or_cber_ind
					.unwrap_or(false)
		}
		RuleCondition::FdaPreAndaNumberRequired => {
			facts.fda_type_of_report_is_two.unwrap_or(false)
				&& facts
					.fda_msg_receiver_is_cder_ind_exempt_ba_be
					.unwrap_or(false)
		}
		RuleCondition::FdaCrossReportedIndRequired => {
			facts.fda_has_ind_number.unwrap_or(false)
		}
		RuleCondition::FdaPreAndaRequired => {
			facts.fda_type_of_report_is_two.unwrap_or(false)
				&& facts
					.fda_msg_receiver_is_cder_ind_exempt_ba_be
					.unwrap_or(false)
				&& !facts.fda_has_pre_anda.unwrap_or(false)
		}
		RuleCondition::FdaPreAndaForbidden => {
			facts.fda_has_pre_anda.unwrap_or(false)
				&& facts.fda_batch_receiver_is_zzfda.unwrap_or(false)
				&& facts.fda_msg_receiver_is_cder_or_cber.unwrap_or(false)
		}
		RuleCondition::FdaGk10aRequired => facts.fda_has_pre_anda.unwrap_or(false),
		RuleCondition::FdaPremarketReportTypeMustBeTwo => {
			facts.fda_batch_receiver_is_zzfda_premarket.unwrap_or(false)
				&& facts.fda_msg_receiver_is_premarket.unwrap_or(false)
				&& facts.fda_has_pre_anda.unwrap_or(false)
				&& facts.fda_study_type_is_1_2_3.unwrap_or(false)
		}
		RuleCondition::MfdsRelatednessSourcePresent => {
			facts.mfds_relatedness_source_present.unwrap_or(false)
		}
		RuleCondition::MfdsRelatednessMethodRequiredContext => facts
			.mfds_relatedness_method_required_context
			.unwrap_or(false),
		RuleCondition::MfdsRelatednessKr1RequiredContext => {
			facts.mfds_relatedness_kr1_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsRelatednessKr2RequiredContext => {
			facts.mfds_relatedness_kr2_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsRelatednessMethodOrResultPresent => {
			facts.mfds_relatedness_method_present.unwrap_or(false)
				|| facts.mfds_relatedness_result_present.unwrap_or(false)
		}
		RuleCondition::MfdsProductCodeRequiredContext => {
			facts.mfds_product_code_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsProductVersionRequiredContext => {
			facts.mfds_product_version_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsSubstanceCodeRequiredContext => {
			facts.mfds_substance_code_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsSubstanceVersionRequiredContext => facts
			.mfds_substance_version_required_context
			.unwrap_or(false),
		RuleCondition::MfdsPastDrugCodeRequiredContext => {
			facts.mfds_past_drug_code_required_context.unwrap_or(false)
		}
		RuleCondition::MfdsPastDrugVersionRequiredContext => facts
			.mfds_past_drug_version_required_context
			.unwrap_or(false),
		RuleCondition::MfdsParentPastDrugCodeRequiredContext => facts
			.mfds_parent_past_drug_code_required_context
			.unwrap_or(false),
		RuleCondition::MfdsParentPastDrugVersionRequiredContext => facts
			.mfds_parent_past_drug_version_required_context
			.unwrap_or(false),
		RuleCondition::MfdsDrugDomesticKr => {
			facts.mfds_drug_domestic_kr.unwrap_or(false)
		}
		RuleCondition::MfdsDrugForeignNonKr => {
			facts.mfds_drug_foreign_non_kr.unwrap_or(false)
		}
		RuleCondition::MfdsSenderTypeDisallowed => {
			facts.mfds_sender_type_disallowed.unwrap_or(false)
		}
		RuleCondition::MfdsPrimarySourceQualificationIsThree => facts
			.mfds_primary_source_qualification_is_three
			.unwrap_or(false),
		RuleCondition::MfdsStudyTypeReactionIsThree => {
			facts.mfds_study_type_reaction_is_three.unwrap_or(false)
		}
		RuleCondition::IchTestPayloadPresent => {
			facts.ich_test_payload_present.unwrap_or(false)
		}
	}
}

pub fn is_rule_value_valid(
	code: &str,
	value_code: Option<&str>,
	null_flavor: Option<&str>,
	facts: RuleFacts,
) -> bool {
	let policy = VALUE_POLICY_BINDINGS
		.iter()
		.find(|binding| binding.code == code)
		.map(|binding| binding.policy);
	match policy {
		Some(ValuePolicy::NonEmpty) => {
			value_code.map(|v| !v.trim().is_empty()).unwrap_or(false)
		}
		Some(ValuePolicy::NonEmptyOrNullFlavor) => {
			let has_value =
				value_code.map(|v| !v.trim().is_empty()).unwrap_or(false);
			has_value || null_flavor.is_some()
		}
		Some(ValuePolicy::SixDigitsNumeric) => value_code
			.map(str::trim)
			.map(|v| v.len() == 6 && v.chars().all(|ch| ch.is_ascii_digit()))
			.unwrap_or(false),
		Some(ValuePolicy::FdaRaceCodeOrNullFlavor) => {
			let value = value_code.map(str::trim).filter(|v| !v.is_empty());
			let code_ok = matches!(
				value,
				Some("C16352" | "C41259" | "C41260" | "C41219" | "C41261")
			);
			code_ok || null_flavor.is_some()
		}
		Some(ValuePolicy::FdaEthnicityCodeOrNullFlavor) => {
			let value = value_code.map(str::trim).filter(|v| !v.is_empty());
			let code_ok = matches!(value, Some("C17459" | "C41222"));
			code_ok || null_flavor.is_some()
		}
		Some(ValuePolicy::FdaGk10aCodeOrNa) => {
			let code_ok = value_code.map(|v| v == "1" || v == "2").unwrap_or(false);
			let null_ok = null_flavor.map(|v| v == "NA").unwrap_or(false);
			code_ok || null_ok
		}
		Some(ValuePolicy::FdaLocalCriteriaCodeByFacts) => {
			let comb_true = facts.fda_combination_product_true.unwrap_or(false);
			let criteria_true = facts.fda_fulfil_expedited_criteria.unwrap_or(false);
			let allowed: &[&str] = if comb_true && criteria_true {
				&["1", "4"]
			} else if comb_true && !criteria_true {
				&["2", "5"]
			} else if !comb_true && criteria_true {
				&["1"]
			} else {
				&["2"]
			};
			value_code
				.map(|code| allowed.contains(&code))
				.unwrap_or(false)
		}
		Some(ValuePolicy::IchC13ConditionalMustBeTwo) => {
			let applies =
				is_rule_condition_satisfied("ICH.C.1.3.CONDITIONAL", facts);
			if !applies {
				return true;
			}
			value_code.map(|v| v == "2").unwrap_or(false)
		}
		None => true,
	}
}

pub fn is_rule_presence_valid(code: &str, present: bool, _facts: RuleFacts) -> bool {
	if REQUIRED_PRESENCE_BINDINGS
		.iter()
		.any(|binding| binding.code == code)
	{
		present
	} else {
		true
	}
}

pub fn should_clear_null_flavor_on_value(code: &str) -> bool {
	has_export_directive(code, ExportDirective::ClearNullFlavorWhenValuePresent)
}

pub fn export_directive_for_rule(code: &str) -> Option<ExportDirective> {
	find_canonical_rule(code).and_then(|rule| rule.export_directive)
}

pub fn has_export_directive(code: &str, directive: ExportDirective) -> bool {
	export_directive_for_rule(code) == Some(directive)
}

pub fn export_normalization_spec_for_rule(
	code: &str,
) -> Option<ExportNormalizationSpec> {
	match code {
		"ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED" => Some(ExportNormalizationSpec {
			xpath: "//hl7:value[@codeSystem='2.16.840.1.113883.6.163']",
			attribute: "code",
			kind: ExportNormalizeKind::AsciiDigitsLen(8),
		}),
		"ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED" => Some(ExportNormalizationSpec {
			xpath: "//hl7:code[@codeSystem='1.0.3166.1.2.2']",
			attribute: "code",
			kind: ExportNormalizeKind::AsciiUpperLen(2),
		}),
		_ => None,
	}
}

pub fn export_xpath_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.XML.RACE.NI.PRUNE" => Some("//hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value[@code='NI']"),
		"ICH.XML.RACE.EMPTY.PRUNE" => Some("//hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value[not(@code) or @nullFlavor]"),
		"ICH.XML.GK11.EMPTY.PRUNE" => Some("//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='2'] and (not(hl7:observation/hl7:value) or normalize-space(hl7:observation/hl7:value)='')]"),
		"ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN" => {
			Some("//hl7:document/hl7:text[@compression]")
		}
		"ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN" => Some(
			"//hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value[@language='JA']",
		),
		"FDA.E.i.3.2h.REQUIRED" => Some("//hl7:observation[hl7:code[@code='7']]/hl7:value"),
		_ => None,
	}
}

pub fn export_xpaths_for_rule(code: &str) -> &'static [&'static str] {
	match code {
		"ICH.XML.PLACEHOLDER.VALUE.PRUNE" => &[
			"//hl7:observation/hl7:value[@code='G.k.10.r']",
			"//hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value[@code='C.1.11.1']",
			"//hl7:observation/hl7:value[@code='D.2.3']",
			"//hl7:observation/hl7:value[@unit='D.2.2.1b']",
		],
		"ICH.XML.STRUCTURAL.EMPTY.PRUNE" => &[
			"//hl7:outboundRelationship2",
			"//hl7:component",
			"//hl7:component1",
			"//hl7:subjectOf2",
			"//hl7:observation",
			"//hl7:organizer",
		],
		_ => &[],
	}
}

pub fn export_attribute_strip_spec_for_rule(
	code: &str,
) -> Option<ExportAttributeStripSpec> {
	match code {
		"ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE" => {
			Some(ExportAttributeStripSpec {
				xpath: "//hl7:observation/hl7:value[@codeSystemVersion='D.8.r.6a' or @codeSystemVersion='D.8.r.7a' or @codeSystemVersion='D.9.2.r.1a' or @codeSystemVersion='D.9.4.r.1a']",
				attribute: "codeSystemVersion",
			})
		}
		"ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN" => {
			Some(ExportAttributeStripSpec {
				xpath: "//hl7:document/hl7:text[@compression]",
				attribute: "compression",
			})
		}
		"ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN" => Some(ExportAttributeStripSpec {
			xpath: "//hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value[@language='JA']",
			attribute: "language",
		}),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashSet;

	#[test]
	fn canonical_lookup_covers_validation_rules() {
		for rule in VALIDATION_RULES {
			let canonical = find_canonical_rule(rule.code);
			assert!(canonical.is_some(), "missing canonical rule: {}", rule.code);
		}
	}

	#[test]
	fn no_duplicate_rule_triples() {
		let mut seen = HashSet::new();
		for rule in VALIDATION_RULES {
			let key = (rule.code, rule.profile.as_str(), rule.section);
			assert!(seen.insert(key), "duplicate rule triple: {:?}", key);
		}
	}

	#[test]
	fn duplicate_codes_resolve_by_phase() {
		let case_rule = find_canonical_rule_for_phase(
			"FDA.C.1.7.1.REQUIRED",
			ValidationPhase::CaseValidate,
		)
		.expect("case-phase rule should exist");
		let import_rule = find_canonical_rule_for_phase(
			"FDA.C.1.7.1.REQUIRED",
			ValidationPhase::Import,
		)
		.expect("import-phase rule should exist");
		assert_eq!(case_rule.section, "case-identification");
		assert_eq!(import_rule.section, "xml");
		assert!(case_rule.blocking);
		assert!(import_rule.blocking);
	}

	#[test]
	fn export_only_rules_do_not_apply_to_import_phase() {
		let import_rule = find_canonical_rule_for_phase(
			"ICH.XML.STRUCTURAL.EMPTY.PRUNE",
			ValidationPhase::Import,
		);
		let export_rule = find_canonical_rule_for_phase(
			"ICH.XML.STRUCTURAL.EMPTY.PRUNE",
			ValidationPhase::Export,
		);
		assert!(import_rule.is_none());
		assert!(export_rule.is_some());
	}

	#[test]
	fn migrated_export_directives_are_available() {
		assert_eq!(
			find_canonical_rule("ICH.E.i.7.REQUIRED")
				.and_then(|rule| rule.export_directive),
			None
		);
		assert_eq!(
			find_canonical_rule("FDA.E.i.3.2h.REQUIRED")
				.and_then(|rule| rule.export_directive),
			Some(ExportDirective::RequiredInterventionNullFlavorNi)
		);
		assert_eq!(
			find_canonical_rule("FDA.C.1.7.1.REQUIRED")
				.and_then(|rule| rule.export_directive),
			Some(ExportDirective::ClearNullFlavorWhenValuePresent)
		);
	}

	#[test]
	fn canonical_value_rules_cover_core_ich_required_fields() {
		assert!(!is_rule_value_valid(
			"ICH.C.1.1.REQUIRED",
			Some(""),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"ICH.C.1.1.REQUIRED",
			Some("CASE-001"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"ICH.C.1.3.REQUIRED",
			Some(""),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"ICH.C.1.3.REQUIRED",
			Some("1"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"ICH.C.1.4.REQUIRED",
			Some(""),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"ICH.C.1.4.REQUIRED",
			Some("20260226"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"ICH.C.1.5.REQUIRED",
			Some(""),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"ICH.C.1.5.REQUIRED",
			Some("20260226"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"ICH.E.i.7.REQUIRED",
			None,
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"ICH.E.i.7.REQUIRED",
			Some("3"),
			None,
			RuleFacts::default()
		));
	}

	#[test]
	fn canonical_profile_rules_include_ich_plus_profile_specific() {
		let fda_rules = canonical_rules_for_profile(ValidationProfile::Fda);
		assert!(fda_rules
			.iter()
			.any(|rule| rule.code == "ICH.E.i.7.REQUIRED"));
		assert!(fda_rules
			.iter()
			.any(|rule| rule.code == "FDA.E.i.3.2h.REQUIRED"));
		assert!(!fda_rules
			.iter()
			.any(|rule| rule.code == "MFDS.C.1.7.1.REQUIRED"));
	}

	#[test]
	fn fda_condition_rules_are_evaluated_from_catalog() {
		assert!(!is_rule_condition_satisfied(
			"FDA.C.1.7.1.REQUIRED",
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.1.7.1.REQUIRED",
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_condition_satisfied(
			"FDA.C.2.r.2.EMAIL.REQUIRED",
			RuleFacts {
				fda_primary_source_present: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.2.r.2.EMAIL.REQUIRED",
			RuleFacts {
				fda_primary_source_present: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.5.5a.REQUIRED",
			RuleFacts {
				fda_type_of_report_is_one_or_two: Some(true),
				fda_msg_receiver_is_cder_ind_or_cber_ind: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.5.6.r.REQUIRED",
			RuleFacts {
				fda_has_ind_number: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_condition_satisfied(
			"FDA.D.11.REQUIRED",
			RuleFacts {
				fda_patient_payload_present: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.D.11.REQUIRED",
			RuleFacts {
				fda_patient_payload_present: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_condition_satisfied(
			"MFDS.G.k.9.i.2.r.1.REQUIRED",
			RuleFacts {
				mfds_relatedness_method_present: Some(false),
				mfds_relatedness_result_present: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.G.k.9.i.2.r.1.REQUIRED",
			RuleFacts {
				mfds_relatedness_method_present: Some(true),
				mfds_relatedness_result_present: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
			RuleFacts {
				mfds_relatedness_method_required_context: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.G.k.2.1.KR.1b.REQUIRED",
			RuleFacts {
				mfds_product_code_required_context: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
			RuleFacts {
				mfds_relatedness_kr1_required_context: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_condition_satisfied(
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
			RuleFacts {
				mfds_relatedness_kr2_required_context: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
			RuleFacts {
				mfds_drug_domestic_kr: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_condition_satisfied(
			"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
			RuleFacts {
				mfds_drug_domestic_kr: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED",
			RuleFacts {
				mfds_drug_foreign_non_kr: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.C.3.1.KR.1.REQUIRED",
			RuleFacts {
				mfds_sender_type_disallowed: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.5.5b.REQUIRED",
			RuleFacts {
				fda_type_of_report_is_two: Some(true),
				fda_msg_receiver_is_cder_ind_exempt_ba_be: Some(true),
				fda_has_pre_anda: Some(false),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.C.5.5b.FORBIDDEN",
			RuleFacts {
				fda_has_pre_anda: Some(true),
				fda_batch_receiver_is_zzfda: Some(true),
				fda_msg_receiver_is_cder_or_cber: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"FDA.G.k.10a.REQUIRED",
			RuleFacts {
				fda_has_pre_anda: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"ICH.C.1.3.CONDITIONAL",
			RuleFacts {
				fda_batch_receiver_is_zzfda_premarket: Some(true),
				fda_msg_receiver_is_premarket: Some(true),
				fda_has_pre_anda: Some(true),
				fda_study_type_is_1_2_3: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"ICH.C.1.9.1.CONDITIONAL",
			RuleFacts {
				ich_case_history_true_missing_prior_ids: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"ICH.D.7.2.CONDITIONAL",
			RuleFacts {
				ich_medical_history_missing_d72_text: Some(true),
				..RuleFacts::default()
			}
		));
	}

	#[test]
	fn fda_value_rules_are_evaluated_from_catalog() {
		assert!(is_rule_value_valid(
			"FDA.C.1.12.REQUIRED",
			Some("true"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.C.1.12.REQUIRED",
			None,
			Some("NI"),
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.C.1.12.REQUIRED",
			None,
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.C.5.5a.REQUIRED",
			Some("123456"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.C.5.5a.REQUIRED",
			Some("ABC123"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.C.5.5b.REQUIRED",
			Some("234567"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.D.11.REQUIRED",
			Some("C41260"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.D.12.REQUIRED",
			Some("C41222"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.D.11.REQUIRED",
			Some("1"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.D.12.REQUIRED",
			Some("1"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.D.11.REQUIRED",
			None,
			Some("NI"),
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
			Some("MPID-123"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
			Some(""),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.G.k.10a.REQUIRED",
			Some("1"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.G.k.10a.REQUIRED",
			None,
			Some("NA"),
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.G.k.10a.REQUIRED",
			Some("3"),
			None,
			RuleFacts::default()
		));
		assert!(is_rule_value_valid(
			"FDA.C.1.7.1.REQUIRED",
			Some("4"),
			None,
			RuleFacts {
				fda_combination_product_true: Some(true),
				fda_fulfil_expedited_criteria: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_value_valid(
			"FDA.C.1.7.1.REQUIRED",
			Some("5"),
			None,
			RuleFacts {
				fda_combination_product_true: Some(false),
				fda_fulfil_expedited_criteria: Some(true),
				..RuleFacts::default()
			}
		));
	}

	#[test]
	fn fda_presence_rules_are_evaluated_from_catalog() {
		assert!(is_rule_presence_valid(
			"FDA.N.1.4.REQUIRED",
			true,
			RuleFacts::default()
		));
		assert!(!is_rule_presence_valid(
			"FDA.N.1.4.REQUIRED",
			false,
			RuleFacts::default()
		));
		assert!(is_rule_presence_valid(
			"FDA.C.2.r.2.EMAIL.REQUIRED",
			true,
			RuleFacts::default()
		));
		assert!(!is_rule_presence_valid(
			"FDA.C.2.r.2.EMAIL.REQUIRED",
			false,
			RuleFacts::default()
		));
	}

	#[test]
	fn exporter_null_flavor_clear_directive_is_catalog_driven() {
		assert!(should_clear_null_flavor_on_value("FDA.C.1.7.1.REQUIRED"));
		assert!(should_clear_null_flavor_on_value("FDA.C.1.12.REQUIRED"));
		assert!(!should_clear_null_flavor_on_value("ICH.E.i.7.REQUIRED"));
	}
}
