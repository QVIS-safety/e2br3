use lib_core::regulatory::RegulatoryAuthority;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ValidationRuleMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub section: &'static str,
	pub blocking: bool,
	pub message: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct MaxLengthRuleMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub max_length: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct AllowedValueRuleMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub source_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedValueConstraintKind {
	CodeSet,
	Boolean,
	TrueMarker,
	Numeric,
	Format,
	Vocabulary,
	Descriptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericShape {
	Decimal,
	Integer,
	DottedVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatName {
	E2bDatetime,
	Base64,
	IchIdentifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyScope {
	All,
	Time,
	Gestation,
	Dose,
	Frequency,
	DoseForm,
	Route,
	ItemSeq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierProfile {
	Mpid,
	Phpid,
	SubstanceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintEnforcement {
	CaseValidate,
	RepresentationEnforced,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AllowedValueConstraint {
	pub kind: AllowedValueConstraintKind,
	#[serde(default)]
	pub values: Vec<String>,
	pub numeric_shape: Option<NumericShape>,
	pub format_name: Option<FormatName>,
	pub vocabulary_scope: Option<VocabularyScope>,
	pub identifier_profile: Option<IdentifierProfile>,
	pub enforcement: Option<ConstraintEnforcement>,
}

#[derive(Debug, Deserialize)]
struct EmbeddedDictionary {
	entries: Vec<EmbeddedDictionaryEntry>,
}

#[derive(Debug, Deserialize)]
struct EmbeddedDictionaryEntry {
	code: String,
	allowed_value_constraint: Option<AllowedValueConstraint>,
	#[serde(default)]
	null_flavors: Vec<String>,
}

static EMBEDDED_ICH_DICTIONARY: OnceLock<EmbeddedDictionary> = OnceLock::new();
static ALLOWED_VALUE_CONSTRAINTS: OnceLock<HashMap<String, AllowedValueConstraint>> =
	OnceLock::new();

fn embedded_ich_dictionary() -> &'static EmbeddedDictionary {
	EMBEDDED_ICH_DICTIONARY.get_or_init(|| {
		serde_json::from_str(include_str!(
			"../../../../registry/dictionary/ich-e2br3.json"
		))
		.expect("embedded ICH dictionary should parse")
	})
}

fn allowed_value_constraints() -> &'static HashMap<String, AllowedValueConstraint> {
	ALLOWED_VALUE_CONSTRAINTS.get_or_init(|| {
		embedded_ich_dictionary()
			.entries
			.iter()
			.filter_map(|entry| {
				entry.allowed_value_constraint.clone().map(|constraint| {
					(format!("ICH.{}.ALLOWED.VALUE", entry.code), constraint)
				})
			})
			.collect()
	})
}

#[derive(Debug, Clone, Copy)]
pub struct VocabularyRuleMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub vocabulary: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct VocabularyVariantMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub receiver: &'static str,
	pub vocabulary: &'static str,
	pub scope: VocabularyScope,
}

#[derive(Debug, Clone, Copy)]
pub struct NullFlavorRuleMetadata {
	pub code: &'static str,
	pub authority: RegulatoryAuthority,
	pub source_hash: u64,
}

#[path = "catalog_dictionary_constraints.rs"]
mod catalog_dictionary_constraints;
pub use catalog_dictionary_constraints::{
	ALLOWED_VALUE_RULES, ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES,
	NULL_FLAVOR_RULES, VOCABULARY_RULES, VOCABULARY_VARIANTS,
};

macro_rules! max_length_rules {
	($(($code:literal, $authority:ident, $max_length:literal),)*) => {
		&[
			$(MaxLengthRuleMetadata {
				code: $code,
				authority: RegulatoryAuthority::$authority,
				max_length: $max_length,
			},)*
		]
	};
}

pub const MAX_LENGTH_RULES: &[MaxLengthRuleMetadata] = max_length_rules![
	("ICH.N.1.1.LENGTH.MAX", Ich, 2),
	("ICH.N.1.2.LENGTH.MAX", Ich, 100),
	("ICH.N.1.3.LENGTH.MAX", Ich, 60),
	("ICH.N.1.4.LENGTH.MAX", Ich, 60),
	("ICH.N.2.r.1.LENGTH.MAX", Ich, 100),
	("ICH.N.2.r.2.LENGTH.MAX", Ich, 60),
	("ICH.N.2.r.3.LENGTH.MAX", Ich, 60),
	("ICH.C.1.1.LENGTH.MAX", Ich, 100),
	("ICH.C.1.3.LENGTH.MAX", Ich, 1),
	("ICH.C.1.6.1.r.1.LENGTH.MAX", Ich, 2000),
	("ICH.C.1.8.1.LENGTH.MAX", Ich, 100),
	("ICH.C.1.8.2.LENGTH.MAX", Ich, 1),
	("ICH.C.1.9.1.r.1.LENGTH.MAX", Ich, 100),
	("ICH.C.1.9.1.r.2.LENGTH.MAX", Ich, 100),
	("ICH.C.1.10.r.LENGTH.MAX", Ich, 100),
	("ICH.C.1.11.1.LENGTH.MAX", Ich, 1),
	("ICH.C.1.11.2.LENGTH.MAX", Ich, 2000),
	("ICH.C.2.r.1.1.LENGTH.MAX", Ich, 50),
	("ICH.C.2.r.1.2.LENGTH.MAX", Ich, 60),
	("ICH.C.2.r.1.3.LENGTH.MAX", Ich, 60),
	("ICH.C.2.r.1.4.LENGTH.MAX", Ich, 60),
	("ICH.C.2.r.2.1.LENGTH.MAX", Ich, 60),
	("ICH.C.2.r.2.2.LENGTH.MAX", Ich, 60),
	("ICH.C.2.r.2.3.LENGTH.MAX", Ich, 100),
	("ICH.C.2.r.2.4.LENGTH.MAX", Ich, 35),
	("ICH.C.2.r.2.5.LENGTH.MAX", Ich, 40),
	("ICH.C.2.r.2.6.LENGTH.MAX", Ich, 15),
	("ICH.C.2.r.2.7.LENGTH.MAX", Ich, 33),
	("ICH.C.2.r.3.LENGTH.MAX", Ich, 2),
	("ICH.C.2.r.4.LENGTH.MAX", Ich, 1),
	("ICH.C.2.r.5.LENGTH.MAX", Ich, 1),
	("ICH.C.3.1.LENGTH.MAX", Ich, 1),
	("ICH.C.3.2.LENGTH.MAX", Ich, 100),
	("ICH.C.3.3.1.LENGTH.MAX", Ich, 60),
	("ICH.C.3.3.2.LENGTH.MAX", Ich, 50),
	("ICH.C.3.3.3.LENGTH.MAX", Ich, 60),
	("ICH.C.3.3.4.LENGTH.MAX", Ich, 60),
	("ICH.C.3.3.5.LENGTH.MAX", Ich, 60),
	("ICH.C.3.4.1.LENGTH.MAX", Ich, 100),
	("ICH.C.3.4.2.LENGTH.MAX", Ich, 35),
	("ICH.C.3.4.3.LENGTH.MAX", Ich, 40),
	("ICH.C.3.4.4.LENGTH.MAX", Ich, 15),
	("ICH.C.3.4.5.LENGTH.MAX", Ich, 2),
	("ICH.C.3.4.6.LENGTH.MAX", Ich, 33),
	("ICH.C.3.4.7.LENGTH.MAX", Ich, 33),
	("ICH.C.3.4.8.LENGTH.MAX", Ich, 100),
	("ICH.C.4.r.1.LENGTH.MAX", Ich, 500),
	("ICH.C.5.1.r.1.LENGTH.MAX", Ich, 50),
	("ICH.C.5.1.r.2.LENGTH.MAX", Ich, 2),
	("ICH.C.5.2.LENGTH.MAX", Ich, 2000),
	("ICH.C.5.3.LENGTH.MAX", Ich, 50),
	("ICH.C.5.4.LENGTH.MAX", Ich, 1),
	("ICH.D.1.LENGTH.MAX", Ich, 60),
	("ICH.D.1.1.1.LENGTH.MAX", Ich, 20),
	("ICH.D.1.1.2.LENGTH.MAX", Ich, 20),
	("ICH.D.1.1.3.LENGTH.MAX", Ich, 20),
	("ICH.D.1.1.4.LENGTH.MAX", Ich, 20),
	("ICH.D.2.2a.LENGTH.MAX", Ich, 5),
	("ICH.D.2.2b.LENGTH.MAX", Ich, 50),
	("ICH.D.2.2.1a.LENGTH.MAX", Ich, 3),
	("ICH.D.2.2.1b.LENGTH.MAX", Ich, 50),
	("ICH.D.2.3.LENGTH.MAX", Ich, 1),
	("ICH.D.3.LENGTH.MAX", Ich, 6),
	("ICH.D.4.LENGTH.MAX", Ich, 3),
	("ICH.D.5.LENGTH.MAX", Ich, 1),
	("ICH.D.7.1.r.1a.LENGTH.MAX", Ich, 4),
	("ICH.D.7.1.r.1b.LENGTH.MAX", Ich, 8),
	("ICH.D.7.1.r.5.LENGTH.MAX", Ich, 2000),
	("ICH.D.7.2.LENGTH.MAX", Ich, 10000),
	("ICH.D.8.r.1.LENGTH.MAX", Ich, 250),
	("ICH.D.8.r.2a.LENGTH.MAX", Ich, 10),
	("ICH.D.8.r.2b.LENGTH.MAX", Ich, 1000),
	("ICH.D.8.r.3a.LENGTH.MAX", Ich, 10),
	("ICH.D.8.r.3b.LENGTH.MAX", Ich, 250),
	("ICH.D.8.r.6a.LENGTH.MAX", Ich, 4),
	("ICH.D.8.r.6b.LENGTH.MAX", Ich, 8),
	("ICH.D.8.r.7a.LENGTH.MAX", Ich, 4),
	("ICH.D.8.r.7b.LENGTH.MAX", Ich, 8),
	("ICH.D.9.2.r.1a.LENGTH.MAX", Ich, 4),
	("ICH.D.9.2.r.1b.LENGTH.MAX", Ich, 8),
	("ICH.D.9.2.r.2.LENGTH.MAX", Ich, 250),
	("ICH.D.9.4.r.1a.LENGTH.MAX", Ich, 4),
	("ICH.D.9.4.r.1b.LENGTH.MAX", Ich, 8),
	("ICH.D.9.4.r.2.LENGTH.MAX", Ich, 250),
	("ICH.D.10.1.LENGTH.MAX", Ich, 60),
	("ICH.D.10.2.2a.LENGTH.MAX", Ich, 3),
	("ICH.D.10.2.2b.LENGTH.MAX", Ich, 50),
	("ICH.D.10.4.LENGTH.MAX", Ich, 6),
	("ICH.D.10.5.LENGTH.MAX", Ich, 3),
	("ICH.D.10.6.LENGTH.MAX", Ich, 1),
	("ICH.D.10.7.1.r.1a.LENGTH.MAX", Ich, 4),
	("ICH.D.10.7.1.r.1b.LENGTH.MAX", Ich, 8),
	("ICH.D.10.7.1.r.5.LENGTH.MAX", Ich, 2000),
	("ICH.D.10.7.2.LENGTH.MAX", Ich, 10000),
	("ICH.D.10.8.r.1.LENGTH.MAX", Ich, 250),
	("ICH.D.10.8.r.2a.LENGTH.MAX", Ich, 10),
	("ICH.D.10.8.r.2b.LENGTH.MAX", Ich, 1000),
	("ICH.D.10.8.r.3a.LENGTH.MAX", Ich, 10),
	("ICH.D.10.8.r.3b.LENGTH.MAX", Ich, 250),
	("ICH.D.10.8.r.6a.LENGTH.MAX", Ich, 4),
	("ICH.D.10.8.r.6b.LENGTH.MAX", Ich, 8),
	("ICH.D.10.8.r.7a.LENGTH.MAX", Ich, 4),
	("ICH.D.10.8.r.7b.LENGTH.MAX", Ich, 8),
	("ICH.E.i.1.1a.LENGTH.MAX", Ich, 250),
	("ICH.E.i.1.1b.LENGTH.MAX", Ich, 3),
	("ICH.E.i.1.2.LENGTH.MAX", Ich, 250),
	("ICH.E.i.2.1a.LENGTH.MAX", Ich, 4),
	("ICH.E.i.2.1b.LENGTH.MAX", Ich, 8),
	("ICH.E.i.3.1.LENGTH.MAX", Ich, 1),
	("ICH.E.i.6a.LENGTH.MAX", Ich, 5),
	("ICH.E.i.6b.LENGTH.MAX", Ich, 50),
	("ICH.E.i.7.LENGTH.MAX", Ich, 1),
	("ICH.E.i.9.LENGTH.MAX", Ich, 2),
	("ICH.F.r.2.1.LENGTH.MAX", Ich, 250),
	("ICH.F.r.2.2a.LENGTH.MAX", Ich, 4),
	("ICH.F.r.2.2b.LENGTH.MAX", Ich, 8),
	("ICH.F.r.3.1.LENGTH.MAX", Ich, 1),
	("ICH.F.r.3.2.LENGTH.MAX", Ich, 50),
	("ICH.F.r.3.3.LENGTH.MAX", Ich, 50),
	("ICH.F.r.3.4.LENGTH.MAX", Ich, 2000),
	("ICH.F.r.4.LENGTH.MAX", Ich, 50),
	("ICH.F.r.5.LENGTH.MAX", Ich, 50),
	("ICH.F.r.6.LENGTH.MAX", Ich, 2000),
	("ICH.G.k.1.LENGTH.MAX", Ich, 1),
	("ICH.G.k.2.1.1a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.2.1.1b.LENGTH.MAX", Ich, 1000),
	("ICH.G.k.2.1.2a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.2.1.2b.LENGTH.MAX", Ich, 250),
	("ICH.G.k.2.2.LENGTH.MAX", Ich, 250),
	("ICH.G.k.2.3.r.1.LENGTH.MAX", Ich, 250),
	("ICH.G.k.2.3.r.2a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.2.3.r.2b.LENGTH.MAX", Ich, 100),
	("ICH.G.k.2.3.r.3a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.2.3.r.3b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.2.4.LENGTH.MAX", Ich, 2),
	("ICH.G.k.3.1.LENGTH.MAX", Ich, 35),
	("ICH.G.k.3.2.LENGTH.MAX", Ich, 2),
	("ICH.G.k.3.3.LENGTH.MAX", Ich, 60),
	("ICH.G.k.4.r.1a.LENGTH.MAX", Ich, 8),
	("ICH.G.k.4.r.1b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.4.r.2.LENGTH.MAX", Ich, 4),
	("ICH.G.k.4.r.3.LENGTH.MAX", Ich, 50),
	("ICH.G.k.4.r.6a.LENGTH.MAX", Ich, 5),
	("ICH.G.k.4.r.6b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.4.r.7.LENGTH.MAX", Ich, 35),
	("ICH.G.k.4.r.8.LENGTH.MAX", Ich, 2000),
	("ICH.G.k.4.r.9.1.LENGTH.MAX", Ich, 60),
	("ICH.G.k.4.r.9.2a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.4.r.9.2b.LENGTH.MAX", Ich, 100),
	("ICH.G.k.4.r.10.1.LENGTH.MAX", Ich, 60),
	("ICH.G.k.4.r.10.2a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.4.r.10.2b.LENGTH.MAX", Ich, 100),
	("ICH.G.k.4.r.11.1.LENGTH.MAX", Ich, 60),
	("ICH.G.k.4.r.11.2a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.4.r.11.2b.LENGTH.MAX", Ich, 100),
	("ICH.G.k.5a.LENGTH.MAX", Ich, 10),
	("ICH.G.k.5b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.6a.LENGTH.MAX", Ich, 3),
	("ICH.G.k.6b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.7.r.1.LENGTH.MAX", Ich, 250),
	("ICH.G.k.7.r.2a.LENGTH.MAX", Ich, 4),
	("ICH.G.k.7.r.2b.LENGTH.MAX", Ich, 8),
	("ICH.G.k.8.LENGTH.MAX", Ich, 1),
	("ICH.G.k.9.i.2.r.1.LENGTH.MAX", Ich, 60),
	("ICH.G.k.9.i.2.r.2.LENGTH.MAX", Ich, 60),
	("ICH.G.k.9.i.2.r.3.LENGTH.MAX", Ich, 60),
	("ICH.G.k.9.i.3.1a.LENGTH.MAX", Ich, 5),
	("ICH.G.k.9.i.3.1b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.9.i.3.2a.LENGTH.MAX", Ich, 5),
	("ICH.G.k.9.i.3.2b.LENGTH.MAX", Ich, 50),
	("ICH.G.k.9.i.4.LENGTH.MAX", Ich, 1),
	("ICH.G.k.10.r.LENGTH.MAX", Ich, 2),
	("ICH.G.k.11.LENGTH.MAX", Ich, 2000),
	("ICH.H.1.LENGTH.MAX", Ich, 100000),
	("ICH.H.2.LENGTH.MAX", Ich, 20000),
	("ICH.H.3.r.1a.LENGTH.MAX", Ich, 4),
	("ICH.H.3.r.1b.LENGTH.MAX", Ich, 8),
	("ICH.H.4.LENGTH.MAX", Ich, 20000),
	("ICH.H.5.r.1a.LENGTH.MAX", Ich, 100000),
	("ICH.H.5.r.1b.LENGTH.MAX", Ich, 3),
	("FDA.C.1.7.1.LENGTH.MAX", Fda, 1),
	("FDA.C.2.r.2.8.LENGTH.MAX", Fda, 100),
	("FDA.C.5.5a.LENGTH.MAX", Fda, 10),
	("FDA.C.5.5b.LENGTH.MAX", Fda, 10),
	("FDA.C.5.6.r.LENGTH.MAX", Fda, 10),
	("FDA.D.11.r.1.LENGTH.MAX", Fda, 10),
	("FDA.D.12.LENGTH.MAX", Fda, 10),
	("FDA.G.k.1.a.LENGTH.MAX", Fda, 1),
	("FDA.G.k.10a.LENGTH.MAX", Fda, 2),
	("FDA.G.k.10.1.LENGTH.MAX", Fda, 10),
	("FDA.G.k.12.r.2.r.LENGTH.MAX", Fda, 1),
	("FDA.G.k.12.r.3.r.LENGTH.MAX", Fda, 7),
	("FDA.G.k.12.r.4.LENGTH.MAX", Fda, 80),
	("FDA.G.k.12.r.5.LENGTH.MAX", Fda, 80),
	("FDA.G.k.12.r.6.LENGTH.MAX", Fda, 10),
	("FDA.G.k.12.r.7.1a.LENGTH.MAX", Fda, 100),
	("FDA.G.k.12.r.7.1b.LENGTH.MAX", Fda, 100),
	("FDA.G.k.12.r.7.1c.LENGTH.MAX", Fda, 35),
	("FDA.G.k.12.r.7.1d.LENGTH.MAX", Fda, 40),
	("FDA.G.k.12.r.7.1e.LENGTH.MAX", Fda, 2),
	("FDA.G.k.12.r.8.LENGTH.MAX", Fda, 1),
	("FDA.G.k.12.r.9.LENGTH.MAX", Fda, 100),
	("FDA.G.k.12.r.10.LENGTH.MAX", Fda, 1),
	("FDA.G.k.12.r.11.r.LENGTH.MAX", Fda, 1),
	("MFDS.C.2.r.4.KR.1.LENGTH.MAX", Mfds, 1),
	("MFDS.C.3.1.KR.1.LENGTH.MAX", Mfds, 1),
	("MFDS.C.5.4.KR.1.LENGTH.MAX", Mfds, 1),
	("MFDS.D.8.r.1.KR.1a.LENGTH.MAX", Mfds, 20),
	("MFDS.D.8.r.1.KR.1b.LENGTH.MAX", Mfds, 10),
	("MFDS.D.10.8.r.1.KR.1a.LENGTH.MAX", Mfds, 20),
	("MFDS.D.10.8.r.1.KR.1b.LENGTH.MAX", Mfds, 10),
	("MFDS.G.k.2.1.KR.1a.LENGTH.MAX", Mfds, 20),
	("MFDS.G.k.2.1.KR.1b.LENGTH.MAX", Mfds, 10),
	("MFDS.G.k.2.3.r.1.KR.1a.LENGTH.MAX", Mfds, 20),
	("MFDS.G.k.2.3.r.1.KR.1b.LENGTH.MAX", Mfds, 10),
	("MFDS.G.k.9.i.2.r.2.KR.1.LENGTH.MAX", Mfds, 1),
	("MFDS.G.k.9.i.2.r.3.KR.1.LENGTH.MAX", Mfds, 1),
	("MFDS.G.k.9.i.2.r.3.KR.2.LENGTH.MAX", Mfds, 1),
];

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
}

impl ValidationPhase {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Import => "import",
			Self::CaseValidate => "case_validate",
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
		authority: RegulatoryAuthority::Fda,
		section: "case-identification",
		blocking: false,
		message: "FDA recommends [C.1.12] combination product report indicator.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.12.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "case-identification",
		blocking: true,
		message: "FDA requires [C.1.12] combination product report indicator.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.12.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.C.1.12 combination product report indicator is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "case-identification",
		blocking: true,
		message: "FDA requires [C.1.7.1] when expedited criteria is fulfilled.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.C.1.7.1 local criteria report type is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.1.7.1.REQUIRED.MISSING_CODE",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.1.7.1 local criteria report type missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.2.r.2.EMAIL.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "reporter",
		blocking: true,
		message: "FDA requires reporter email when primary source is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5a.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.5a] IND Number where AE Occurred when C.1.3 is 1/2 and message receiver is CDER_IND/CBER_IND (6 digits).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.FORBIDDEN",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.5.5b must not be provided for postmarket (N.1.4=ZZFDA, N.2.r.3=CDER/CBER).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.5b] Pre-ANDA Number where AE Occurred when C.1.3 is 2 and message receiver is CDER_IND_EXEMPT_BA_BE (6 digits).",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.5b.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message:
			"FDA.C.5.5b is required when C.1.3=2 and N.2.r.3=CDER_IND_EXEMPT_BA_BE.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.5.6.r.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "study",
		blocking: true,
		message:
			"FDA requires [C.5.6.r] cross reported IND when [C.5.5a] is populated.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.11.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "patient",
		blocking: true,
		message: "FDA requires [D.11] patient race when patient payload is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.11.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.D.11 patient race is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.12.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "patient",
		blocking: true,
		message:
			"FDA requires [D.12] patient ethnicity when patient payload is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.D.12.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.D.12 patient ethnicity is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.E.i.3.2h.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "reactions",
		blocking: true,
		message:
			"FDA requires [E.i.3.2h] when other medically important condition is selected.",
	},
	ValidationRuleMetadata {
		code: "FDA.E.i.3.2h.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.E.i.3.2h required intervention is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.1.A.CONDITIONAL",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA [G.K.1.A]=1 is allowed only when [C.1.12]=true, [G.K.12.r.1]=true, and [G.k.1]=4 for the same product.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.R.11.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: false,
		message:
			"FDA recommends [G.K.12.R.11] when [G.K.12.r.1]=true and [C.1.7.1]=4.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.R.3.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA requires [G.K.12.R.3] when [G.K.12.r.1]=true for postmarket ICSRs.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.K.12.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message:
			"FDA postmarket requires at least one suspect product with [G.K.12.r.1]=true when [C.1.7.1]=5.",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.10a.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.G.k.10a is required when FDA.C.5.5b is present.",
	},
	ValidationRuleMetadata {
		code: "FDA.N.1.4.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "xml",
		blocking: true,
		message: "FDA.N.1.4 batch receiver identifier missing.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.11.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.11.2] Nullification reason is required when [C.1.11.1] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.2] must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.3.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"C.1.3 must be 2 when premarket receiver and FDA.C.5.5b are present with study type 1/2/3.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.3] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.4] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.4] must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.4.AFTER_C.1.2.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.4] cannot be later than [C.1.2].",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.4] cannot be later than [C.1.5].",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.5.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.5] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.5] must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.5.AFTER_C.1.2.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.5] cannot be later than [C.1.2].",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.6.1.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.6.1.r.1] Document description is required when additional documents are available.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.7.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.7] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "C.1.9.1 is true but C.1.9.1.r.1/.r.2 are missing.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.9.1.r.1] Source of the case identifier is required when an other case identifier row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message:
			"[C.1.9.1.r.2] Case identifier is required when an other case identifier row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "Safety report identification is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole/id has extension and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message: "primaryRole/id missing extension; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole/id with root 2.16.840.1.113883.3.989.2.1.3.6 requires extension or nullFlavor.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.2.NAME.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"primaryRole name element has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message: "primaryRole name element is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"representedOrganization/name has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"representedOrganization/name is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message: "[C.2.r.4] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.5.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: false,
		message:
			"[C.2.r.5] one primary source for regulatory purposes should be selected.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.3.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "sender",
		blocking: true,
		message: "[C.3.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.3.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "sender",
		blocking: true,
		message: "[C.3.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "study",
		blocking: true,
		message:
			"[C.5.4] Study type where reaction(s) / event(s) were observed is required when [C.1.3] is report from study (2).",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.2.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message:
			"[C.2.r.2.1] Reporter organization is required when report type is study (C.1.3=2).",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "study",
		blocking: true,
		message:
			"[C.5.3] Sponsor study number is required when report type is study (C.1.3=2).",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "study",
		blocking: true,
		message:
			"researchStudy/title has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "study",
		blocking: true,
		message: "researchStudy/title is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.1] This Element is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.1.1.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.1.1.4] Patient study number is required when report type is study (C.1.3=2).",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.2.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.2.2a] Parent age is required when [D.10.2.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.10.2.1] Parent date of birth must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.2.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.2.2b] Parent age unit is required when [D.10.2.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.6.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.10.6] Parent sex is required when parent data is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.3] Parent last menstrual period date must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.7.1.r.1a] Parent medical history MedDRA version is required when [D.10.7.1.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.7.1.r.1b] Parent medical history MedDRA code is required when [D.10.7.1.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.7.1.r.2/D.10.7.1.r.4] Parent medical history dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.MPID_PHPID.EXCLUSIVE",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.2b/D.10.8.r.3b] Any given parent past drug entry may have either MPID or PhPID, but not both.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.2a] Parent past drug MPID version is required when [D.10.8.r.2b] MPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.3a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.3a] Parent past drug PhPID version is required when [D.10.8.r.3b] PhPID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.4/D.10.8.r.5] Parent past drug dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.6a] Parent past drug indication MedDRA version is required when [D.10.8.r.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.6b] Parent past drug indication MedDRA code is required when [D.10.8.r.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.7a] Parent past drug reaction MedDRA version is required when [D.10.8.r.7b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.10.8.r.7b] Parent past drug reaction MedDRA code is required when [D.10.8.r.7a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2.1a] Gestation period is required when [D.2.2.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2.1b] Gestation period unit is required when [D.2.2.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.2.1] Date of birth must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2a] Age at time of onset is required when [D.2.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.2.2b] Age unit is required when [D.2.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.BIRTHTIME.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"birthTime has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "birthTime missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.6.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.6] Last menstrual period date must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.5.SEX.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "administrativeGenderCode missing code; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.7.1.r.1a] MedDRA version for medical history is required when [D.7.1.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.7.1.r.1b] Medical history MedDRA code is required when [D.7.1.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.7.1.r] Medical history dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.2.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "D.7.2 must be provided when D.7.1.r.1b is not provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.MPID_PHPID.EXCLUSIVE",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.2b/D.8.r.3b] Any given past drug entry may have either MPID or PhPID, but not both.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.8.r.4/D.8.r.5] Past drug dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.6a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.6a] Indication MedDRA version is required when [D.8.r.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.6b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.6b] Indication MedDRA code is required when [D.8.r.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.7a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.7a] Reaction MedDRA version is required when [D.8.r.7b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.7b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.8.r.7b] Reaction MedDRA code is required when [D.8.r.7a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.1a] Reported cause of death MedDRA version is required when [D.9.2.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.9.1] Date of death must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.1b] Reported cause of death MedDRA code is required when [D.9.2.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.2.r.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.2.r.2] Reported cause of death comments are required when [D.9.2.r.1] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.3] Autopsy was performed is required when [D.9.1] date of death is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.1a] Autopsy cause of death MedDRA version is required when [D.9.4.r.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.1b] Autopsy cause of death MedDRA code is required when [D.9.4.r.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.9.4.r.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"[D.9.4.r.2] Autopsy cause of death comments are required when [D.9.4.r.1] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"patient effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson birthTime has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson birthTime missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson name element has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message:
			"associatedPerson name element is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2.CRITERIA.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.3.2] At least one seriousness criterion must be true when [E.i.3.1] is serious.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2.NI.ONLY",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.3.2] Seriousness criteria null flavor must be NI; other null flavor values are not permitted.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.1.1a] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.1.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.1.1b] is required when [E.i.1.1a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.2.1a] Reaction MedDRA version is required when [E.i.2.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.2.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.2.1b] Reaction MedDRA code is required when a reaction row is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"reaction effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.4/E.i.5] Reaction dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.4-6.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: false,
		message: "Reaction should include start, end, or duration.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.6a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.6a] Reaction duration is required when [E.i.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.6b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message:
			"[E.i.6b] Reaction duration unit is required when [E.i.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.7.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.7] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message: "[F.r.1] Test date is required when [F.r.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message: "[F.r.1] Test date must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.1] Test name (free text) is required when [F.r.1] is populated and [F.r.2.2b] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.2a] Test name MedDRA version is required when [F.r.2.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.2.2b] Test name MedDRA code is required when [F.r.1] is populated and [F.r.2.1] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message: "[F.r.2] is required when test payload is present.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.1] Test result (coded) is required when [F.r.2] is populated and neither [F.r.3.2] nor [F.r.3.4] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.2] Test result (value/finding) is required when [F.r.2] is populated and [F.r.3.1] and [F.r.3.4] are not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.3] Test result unit is required when [F.r.3.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.F.r.3.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "tests",
		blocking: true,
		message:
			"[F.r.3.4] Result unstructured data is required when [F.r.2] is populated and [F.r.3] is not populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message: "[G.k.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message: "[G.k.2.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.1] Substance name is required when an active substance row has no TermID.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.2a] Substance TermID version is required when [G.k.2.3.r.2b] TermID is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.2.3.r.3b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.2.3.r.3b] Strength unit is required when [G.k.2.3.r.3a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.10.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.10.2a] Route of administration TermID version is required when [G.k.4.r.10.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"formCode missing code/codeSystem/originalText; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.11.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.11.2a] Parent route TermID version is required when [G.k.4.r.11.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message: "routeCode missing code; originalText or nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.1b] Dose unit is required when [G.k.4.r.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.3] Time interval unit is required when [G.k.4.r.2] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"drug effectiveTime low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.4/G.k.4.r.5] Drug administration dates must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.4-8.CONDITIONAL",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message: "Drug requires start, end, or duration.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.6a] Duration value is required when [G.k.4.r.6b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.6b] Duration unit is required when [G.k.4.r.6a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.4.r.9.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.4.r.9.2a] Dose form TermID version is required when [G.k.4.r.9.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.5a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.5a] Cumulative dose to first reaction value is required when [G.k.5b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.5b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.5b] Cumulative dose to first reaction unit is required when [G.k.5a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.6a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.6a] Gestation period at exposure value is required when [G.k.6b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.6b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.6b] Gestation period at exposure unit is required when [G.k.6a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.7.r.2a] Indication MedDRA version is required when [G.k.7.r.2b] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.7.r.2b] Indication MedDRA code is required when [G.k.7.r.2a] is provided.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.1a] Administration start interval value is required when [G.k.9.i.3.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.1b] Administration start interval unit is required when [G.k.9.i.3.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.2a] Last-dose interval value is required when [G.k.9.i.3.2b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.9.i.3.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message:
			"[G.k.9.i.3.2b] Last-dose interval unit is required when [G.k.9.i.3.2a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "narrative",
		blocking: true,
		message: "[H.1] This Element is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.3.r.1a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.3.r.1a] Sender diagnosis MedDRA version is required when [H.3.r.1b] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.3.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.3.r.1b] Sender diagnosis MedDRA code is required when [H.3.r.1a] is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.H.5.r.1b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "narrative",
		blocking: true,
		message:
			"[H.5.r.1b] Case summary language is required when [H.5.r.1a] summary type is populated.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.2] Batch number is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.3] Batch sender identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.4] Batch receiver identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.5.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.5] Date of batch transmission is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.5.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.5] Date of batch transmission must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.2] Message sender identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.3] Message receiver identifier is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.4.FUTURE_DATE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.4] Message date must not be later than today.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "Message header is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.BL.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"BL value has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.BL.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "BL value missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.CODE.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"code has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.CODE.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"code missing code/codeSystem; nullFlavor is required when originalText is absent.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "ISO country code must be 2 uppercase letters.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "doseQuantity must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.EFFECTIVETIME.WIDTH.REQUIRES_BOUND",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "effectiveTime with width must include low/high.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"investigationCharacteristic BL has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"investigationCharacteristic BL missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.IVL_TS.OPERATOR_A.BOUND_REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_TS operator='A' must include low, high, or width.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.LOW_HIGH.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"low/high has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "low/high missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "MedDRA code must be 8 digits.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.MEDDRA.VERSION.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "MedDRA code requires codeSystemVersion.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PERIOD.VALUE_UNIT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "period must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PIVL_TS.PERIOD.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "PIVL_TS must include period.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "PIVL_TS period must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "Placeholder values are not allowed in XML content or attributes.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.ROOT.ITSVERSION.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "Root ITSVersion must be present and set to XML_1.0.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"Root xsi:schemaLocation must be present and reference the expected root schema.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.SXPR_TS.COMP.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "SXPR_TS must include at least one comp element.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.FORMAT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"telecom value must start with tel:, fax:, or mailto:.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"telecom has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TELECOM.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "telecom missing value; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_PQ must include low/high/center.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "IVL_PQ low/high/center must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "PQ must include value and unit.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "Unsupported test result xsi:type.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TEXT.NULLFLAVOR.FORBIDDEN",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message:
			"text/originalText has value and nullFlavor; nullFlavor must be absent when value present.",
	},
	ValidationRuleMetadata {
		code: "ICH.XML.TEXT.NULLFLAVOR.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "xml",
		blocking: true,
		message: "text/originalText is empty; nullFlavor is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.1.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.1.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.N.2.r.4.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[N.2.r.4] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.6.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.6.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.8.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.8.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.8.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.8.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.1.9.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "case-identification",
		blocking: true,
		message: "[C.1.9.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.C.2.r.3.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reporter",
		blocking: true,
		message: "[C.2.r.3] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.7.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.7.2] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.D.8.r.1.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "patient",
		blocking: true,
		message: "[D.8.r.1] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2a.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2a] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2b.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2b] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2c.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2c] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2d.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2d] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2e.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2e] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.E.i.3.2f.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "reactions",
		blocking: true,
		message: "[E.i.3.2f] is required.",
	},
	ValidationRuleMetadata {
		code: "ICH.G.k.3.2.REQUIRED",
		authority: RegulatoryAuthority::Ich,
		section: "drugs",
		blocking: true,
		message: "[G.k.3.2] is required.",
	},
	ValidationRuleMetadata {
		code: "FDA.C.2.r.2.8.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "reporter",
		blocking: true,
		message: "FDA requires [C.2.r.2.8].",
	},
	ValidationRuleMetadata {
		code: "FDA.D.11.r.1.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "patient",
		blocking: true,
		message: "FDA requires [D.11.r.1].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.1.a.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.1.a].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.1.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.1].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.3.r.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.3.r].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.4.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.4].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.5.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.5].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.6.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.6].",
	},
	ValidationRuleMetadata {
		code: "FDA.G.k.12.r.11.r.REQUIRED",
		authority: RegulatoryAuthority::Fda,
		section: "drugs",
		blocking: true,
		message: "FDA requires [G.k.12.r.11.r].",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.2.r.4.KR.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "reporter",
		blocking: true,
		message:
			"MFDS requires [C.2.r.4.KR.1] when reporter qualification [C.2.r.4] is other health professional (3).",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "case-identification",
		blocking: true,
		message:
			"MFDS requires [C.3.1.KR.1] when sender type [C.3.1] is health professional (3).",
	},
	ValidationRuleMetadata {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "study",
		blocking: true,
		message:
			"MFDS requires [C.5.4.KR.1] when study type [C.5.4] is other studies (3).",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires parent past drug code version [D.10.8.r.1.KR.1a] for FR when [D.10.8.r.1.KR.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires parent past drug code [D.10.8.r.1.KR.1b] for KR/FR receiver authorities.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1a.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires past drug code version [D.8.r.1.KR.1a] for FR when [D.8.r.1.KR.1b] is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1b.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "patient",
		blocking: false,
		message:
			"MFDS requires past drug code [D.8.r.1.KR.1b] for KR/FR receiver authorities.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1a.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires product code version [G.k.2.1.KR.1a] for FR when product code is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1b.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires product code [G.k.2.1.KR.1b] for KR/FR receiver authorities.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires substance code version [G.k.2.3.r.1.KR.1a] for FR when substance code is provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires substance code [G.k.2.3.r.1.KR.1b] for KR/FR when product code is not provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires source of assessment when KR method/result values are provided.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires KR method of assessment when source of assessment is present.",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS requires WHO-UMC result when source is present and method is WHO-UMC (1).",
	},
	ValidationRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS requires KRCT result when source is present, method is KRCT (2), and report is clinical (CT/CU).",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: false,
		message:
			"MFDS domestic cases should provide KR ingredient coding for each active substance.",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message: "MFDS domestic cases require KR product coding for the drug.",
	},
	ValidationRuleMetadata {
		code: "MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
		authority: RegulatoryAuthority::Mfds,
		section: "drugs",
		blocking: true,
		message:
			"MFDS foreign-use products must provide WHO MPID/KR product coding.",
	},
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCondition {
	Always,
	IchDictionaryCondition,
	IchCaseHistoryTrueMissingPriorIds,
	IchMedicalHistoryMissingD72Text,
	IchReportTypeIsStudy,
	IchNullificationCodePresent,
	IchSenderOrganizationRequired,
	IchAgeValuePresent,
	IchAgeUnitPresent,
	IchGestationValuePresent,
	IchGestationUnitPresent,
	IchDateOfDeathPresent,
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
	MfdsSenderTypeIsHealthProfessional,
	MfdsPrimarySourceQualificationIsThree,
	MfdsStudyTypeReactionIsThree,
	IchTestPayloadPresent,
}

impl RuleCondition {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Always => "always",
			Self::IchDictionaryCondition => "ich_dictionary_condition",
			Self::IchCaseHistoryTrueMissingPriorIds => {
				"ich_case_history_true_missing_prior_ids"
			}
			Self::IchMedicalHistoryMissingD72Text => {
				"ich_medical_history_missing_d72_text"
			}
			Self::IchReportTypeIsStudy => "ich_report_type_is_study",
			Self::IchNullificationCodePresent => "ich_nullification_code_present",
			Self::IchSenderOrganizationRequired => {
				"ich_sender_organization_required"
			}
			Self::IchAgeValuePresent => "ich_age_value_present",
			Self::IchAgeUnitPresent => "ich_age_unit_present",
			Self::IchGestationValuePresent => "ich_gestation_value_present",
			Self::IchGestationUnitPresent => "ich_gestation_unit_present",
			Self::IchDateOfDeathPresent => "ich_date_of_death_present",
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
			Self::MfdsSenderTypeIsHealthProfessional => {
				"mfds_sender_type_is_health_professional"
			}
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
	pub ich_nullification_code_present: Option<bool>,
	pub ich_sender_organization_required: Option<bool>,
	pub ich_age_value_present: Option<bool>,
	pub ich_age_unit_present: Option<bool>,
	pub ich_gestation_value_present: Option<bool>,
	pub ich_gestation_unit_present: Option<bool>,
	pub ich_date_of_death_present: Option<bool>,
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
	pub mfds_sender_type_is_health_professional: Option<bool>,
	pub mfds_primary_source_qualification_is_three: Option<bool>,
	pub mfds_study_type_reaction_is_three: Option<bool>,
	pub ich_test_payload_present: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalRule<'a> {
	pub code: &'a str,
	pub authority: RegulatoryAuthority,
	pub section: &'a str,
	pub blocking: bool,
	pub category: RuleCategory,
	pub phases: &'a [ValidationPhase],
	pub severity: RuleSeverity,
	pub message: &'a str,
	pub condition: RuleCondition,
}

const PHASES_CASE_VALIDATE: &[ValidationPhase] = &[ValidationPhase::CaseValidate];
const PHASES_IMPORT_ONLY: &[ValidationPhase] = &[ValidationPhase::Import];
const PHASES_METADATA_ONLY: &[ValidationPhase] = &[];

#[derive(Debug, Clone, Copy)]
struct ConditionBinding {
	code: &'static str,
	condition: RuleCondition,
}

#[derive(Debug, Clone, Copy)]
struct ConditionTextBinding {
	code: &'static str,
	condition_text: &'static str,
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
		code: "ICH.C.2.r.2.1.REQUIRED",
		condition: RuleCondition::IchReportTypeIsStudy,
	},
	ConditionBinding {
		code: "ICH.C.5.3.REQUIRED",
		condition: RuleCondition::IchReportTypeIsStudy,
	},
	ConditionBinding {
		code: "ICH.D.1.1.4.REQUIRED",
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
		code: "ICH.C.1.6.1.r.1.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.C.1.11.2.REQUIRED",
		condition: RuleCondition::IchNullificationCodePresent,
	},
	ConditionBinding {
		code: "ICH.C.2.r.3.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.C.2.r.5.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.C.3.2.REQUIRED",
		condition: RuleCondition::IchSenderOrganizationRequired,
	},
	ConditionBinding {
		code: "ICH.D.2.2a.REQUIRED",
		condition: RuleCondition::IchAgeUnitPresent,
	},
	ConditionBinding {
		code: "ICH.D.2.2b.REQUIRED",
		condition: RuleCondition::IchAgeValuePresent,
	},
	ConditionBinding {
		code: "ICH.D.2.2.1a.REQUIRED",
		condition: RuleCondition::IchGestationUnitPresent,
	},
	ConditionBinding {
		code: "ICH.D.2.2.1b.REQUIRED",
		condition: RuleCondition::IchGestationValuePresent,
	},
	ConditionBinding {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.7.2.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.8.r.1.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.8.r.6a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.8.r.6b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.8.r.7a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.8.r.7b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.9.3.REQUIRED",
		condition: RuleCondition::IchDateOfDeathPresent,
	},
	ConditionBinding {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.9.4.r.2.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.2.2a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.2.2b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.E.i.1.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.E.i.6a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.E.i.6b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.1.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.2.2a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.2.2b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.3.1.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.3.2.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.3.3.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.F.r.3.4.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.3.2.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.4.r.3.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.5a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.5b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.6a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.6b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.9.i.3.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.9.i.3.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.9.i.3.2a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.G.k.9.i.3.2b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.H.3.r.1a.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.H.3.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "ICH.H.5.r.1b.REQUIRED",
		condition: RuleCondition::IchDictionaryCondition,
	},
	ConditionBinding {
		code: "MFDS.C.2.r.4.KR.1.REQUIRED",
		condition: RuleCondition::MfdsPrimarySourceQualificationIsThree,
	},
	ConditionBinding {
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		condition: RuleCondition::MfdsSenderTypeIsHealthProfessional,
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
		code: "MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
		condition: RuleCondition::MfdsDrugForeignNonKr,
	},
];

const ICH_CONDITIONAL_TEXT_BINDINGS: &[ConditionTextBinding] = &[
	ConditionTextBinding {
		code: "ICH.C.1.6.1.r.1.REQUIRED",
		condition_text: "Optional, but required if C.1.6.1 is ‘true’.",
	},
	ConditionTextBinding {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		condition_text: "Optional, but required if C.1.9.1=‘true’.",
	},
	ConditionTextBinding {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		condition_text: "Optional, but required if C.1.9.1= ‘true’.",
	},
	ConditionTextBinding {
		code: "ICH.C.1.11.2.REQUIRED",
		condition_text: "Optional, but required when C.1.11.1 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.C.2.r.3.REQUIRED",
		condition_text: "Optional, but required if C.2.r.5 = 1.",
	},
	ConditionTextBinding {
		code: "ICH.C.2.r.5.REQUIRED",
		condition_text: "Required for one and only one instance of this element.",
	},
	ConditionTextBinding {
		code: "ICH.C.3.2.REQUIRED",
		condition_text:
			"Required if the ‘Sender Type’ (C.3.1) is not coded as 7 (Patient / Consumer).",
	},
	ConditionTextBinding {
		code: "ICH.C.5.4.REQUIRED",
		condition_text: "Optional, but required if C.1.3=2 (Report from study).",
	},
	ConditionTextBinding {
		code: "ICH.D.2.2a.REQUIRED",
		condition_text: "Optional, but required if D.2.2b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.2.2b.REQUIRED",
		condition_text: "Optional, but required if D.2.2a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.2.2.1a.REQUIRED",
		condition_text: "Optional, but required if D.2.2.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.2.2.1b.REQUIRED",
		condition_text: "Optional, but required if D.2.2.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		condition_text: "Optional, but required if D.7.1.r.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		condition_text: "Optional, but required if D.7.1.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.7.2.REQUIRED",
		condition_text: "Optional, but required if Section D.7.1 is null.",
	},
	ConditionTextBinding {
		code: "ICH.D.8.r.1.REQUIRED",
		condition_text:
			"Optional, but required by the schema if any data element in section D.8.r is used.",
	},
	ConditionTextBinding {
		code: "ICH.D.8.r.6a.REQUIRED",
		condition_text: "Optional, but required if D.8.r.6b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.8.r.6b.REQUIRED",
		condition_text: "Optional, but required if D.8.r.6a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.8.r.7a.REQUIRED",
		condition_text: "Optional, but required if D.8.r.7b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.8.r.7b.REQUIRED",
		condition_text: "Optional, but required if D.8.r.7a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		condition_text: "Optional, but required if D.9.2.r.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		condition_text: "Optional, but required if D.9.2.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.3.REQUIRED",
		condition_text: "Optional, but required if D.9.1 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		condition_text: "Optional, but required if D.9.4.r.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		condition_text: "Optional, but required if D.9.4.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.9.4.r.2.REQUIRED",
		condition_text: "Optional, but required if D.9.4.r.1 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.2.2a.REQUIRED",
		condition_text: "Optional, but required if D.10.2.2b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.2.2b.REQUIRED",
		condition_text: "Optional, but required if D.10.2.2a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		condition_text:
			"Optional, but required if D.10.7.1.r.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		condition_text:
			"Optional, but required if D.10.7.1.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		condition_text: "Optional, but required if D.10.8.r.6b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		condition_text: "Optional, but required if D.10.8.r.6a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		condition_text: "Optional, but required if D.10.8.r.7b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		condition_text: "Optional, but required if D.10.8.r.7a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.E.i.1.1b.REQUIRED",
		condition_text: "Optional, but required if E.i.1.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.E.i.6a.REQUIRED",
		condition_text: "Optional, but required if E.i.6b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.E.i.6b.REQUIRED",
		condition_text: "Optional, but required if E.i.6a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.1.REQUIRED",
		condition_text: "Optional, but required if F.r.2 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.2.2a.REQUIRED",
		condition_text: "Optional, but required when F.r.2.2b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.2.2b.REQUIRED",
		condition_text:
			"Optional, but required if F.r.1 is populated and F.r.2.2b is not populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.3.1.REQUIRED",
		condition_text:
			"Optional, but required if F.r.2 is populated, and neither F.r.3.2 nor F.r.3.4 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.3.2.REQUIRED",
		condition_text:
			"Optional, but required if F.r.2 is populated, and F.r.3.1 and F.r.3.4 is not populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.3.3.REQUIRED",
		condition_text: "Optional, but required if F.r.3.2 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.F.r.3.4.REQUIRED",
		condition_text:
			"Optional, but required if F.r.2 is populated, and F.r.3 is not populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.3.2.REQUIRED",
		condition_text: "Optional, but required if G.k.3.1 is provided.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		condition_text: "Optional, but required if G.k.4.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.4.r.3.REQUIRED",
		condition_text: "Optional, but required if G.k.4.r.2 is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		condition_text: "Optional, but required if G.k.4.r.6b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		condition_text: "Optional, but required if G.k.4.r.6a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.5a.REQUIRED",
		condition_text: "Optional, but required if G.k.5b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.5b.REQUIRED",
		condition_text: "Optional, but required if G.k.5a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.6a.REQUIRED",
		condition_text: "Optional, but required if G.k.6b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.6b.REQUIRED",
		condition_text: "Optional, but required if G.k.6a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		condition_text: "Optional, but required if G.k.7.r.2b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		condition_text: "Optional, but required if G.k.7.r.2a is provided.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.9.i.3.1a.REQUIRED",
		condition_text:
			"Optional, but required if G.k.9.i.3.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.9.i.3.1b.REQUIRED",
		condition_text:
			"Optional, but required if G.k.9.i.3.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.9.i.3.2a.REQUIRED",
		condition_text:
			"Optional, but required if G.k.9.i.3.2b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.G.k.9.i.3.2b.REQUIRED",
		condition_text:
			"Optional, but required if G.k.9.i.3.2a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.H.3.r.1a.REQUIRED",
		condition_text: "Optional, but required if H.3.r.1b is populated.",
	},
	ConditionTextBinding {
		code: "ICH.H.3.r.1b.REQUIRED",
		condition_text: "Optional, but required if H.3.r.1a is populated.",
	},
	ConditionTextBinding {
		code: "ICH.H.5.r.1b.REQUIRED",
		condition_text: "Required if H.5.r.1a is populated.",
	},
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValuePolicy {
	NonEmpty,
	NonEmptyOrNullFlavor,
	SixDigitsNumeric,
	FdaRaceCodeOrNullFlavor,
	FdaEthnicityCodeOrNullFlavor,
	FdaGk10aCodeOrNa,
	FdaBooleanStringOrNullFlavor,
	FdaLocalCriteriaAllowedCode,
	MfdsHealthProfessionalTypeKr1,
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
		policy: ValuePolicy::FdaBooleanStringOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.C.1.12.RECOMMENDED",
		policy: ValuePolicy::FdaBooleanStringOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.C.1.7.1.REQUIRED",
		policy: ValuePolicy::FdaLocalCriteriaAllowedCode,
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
		code: "FDA.C.2.r.2.8.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.C.5.6.r.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.D.11.REQUIRED",
		policy: ValuePolicy::FdaRaceCodeOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.D.11.r.1.REQUIRED",
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
		code: "FDA.G.k.12.r.4.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "FDA.G.k.12.r.5.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.1.9.1.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		code: "ICH.C.2.r.2.1.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.2.r.3.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.C.5.3.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.D.1.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.D.1.1.4.REQUIRED",
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
		code: "ICH.D.7.2.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.D.8.r.1.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		code: "ICH.E.i.3.2a.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.3.2b.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.3.2c.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.3.2d.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.3.2e.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.3.2f.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.E.i.7.REQUIRED",
		policy: ValuePolicy::NonEmpty,
	},
	ValuePolicyBinding {
		code: "ICH.F.r.1.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
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
		code: "ICH.F.r.3.2.REQUIRED",
		policy: ValuePolicy::NonEmptyOrNullFlavor,
	},
	ValuePolicyBinding {
		code: "ICH.G.k.1.REQUIRED",
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
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		policy: ValuePolicy::MfdsHealthProfessionalTypeKr1,
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
		code: "MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
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

fn condition_for_code(code: &str) -> RuleCondition {
	CONDITION_BINDINGS
		.iter()
		.find(|binding| binding.code == code)
		.map(|binding| binding.condition)
		.unwrap_or(RuleCondition::Always)
}

pub fn condition_text_for_code(code: &str) -> Option<&'static str> {
	ICH_CONDITIONAL_TEXT_BINDINGS
		.iter()
		.find(|binding| binding.code == code)
		.map(|binding| binding.condition_text)
}

fn to_canonical_rule<'a>(rule: &'a ValidationRuleMetadata) -> CanonicalRule<'a> {
	let category = category_for_rule(rule);
	let phases = phases_for_rule(rule);
	let severity = severity_for_rule(rule);
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: rule.section,
		blocking: severity.is_blocking(),
		category,
		phases,
		severity,
		message: rule.message,
		condition: condition_for_code(rule.code),
	}
}

fn to_canonical_max_length_rule<'a>(
	rule: &'a MaxLengthRuleMetadata,
) -> CanonicalRule<'a> {
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: section_for_rule_code(rule.code),
		blocking: true,
		category: RuleCategory::CaseBusiness,
		phases: phases_for_max_length_rule(rule.code),
		severity: RuleSeverity::Blocking,
		message: "Dictionary max length exceeded.",
		condition: RuleCondition::Always,
	}
}

fn phases_for_max_length_rule(code: &str) -> &'static [ValidationPhase] {
	match code {
		"ICH.C.1.1.LENGTH.MAX"
		| "ICH.C.1.3.LENGTH.MAX"
		| "ICH.C.1.6.1.r.1.LENGTH.MAX"
		| "ICH.C.1.8.1.LENGTH.MAX"
		| "ICH.C.1.8.2.LENGTH.MAX"
		| "ICH.C.1.9.1.r.1.LENGTH.MAX"
		| "ICH.C.1.9.1.r.2.LENGTH.MAX"
		| "ICH.C.1.10.r.LENGTH.MAX"
		| "ICH.C.1.11.1.LENGTH.MAX"
		| "ICH.C.1.11.2.LENGTH.MAX"
		| "ICH.C.2.r.1.1.LENGTH.MAX"
		| "ICH.C.2.r.1.2.LENGTH.MAX"
		| "ICH.C.2.r.1.3.LENGTH.MAX"
		| "ICH.C.2.r.1.4.LENGTH.MAX"
		| "ICH.C.2.r.2.1.LENGTH.MAX"
		| "ICH.C.2.r.2.2.LENGTH.MAX"
		| "ICH.C.2.r.2.3.LENGTH.MAX"
		| "ICH.C.2.r.2.4.LENGTH.MAX"
		| "ICH.C.2.r.2.5.LENGTH.MAX"
		| "ICH.C.2.r.2.6.LENGTH.MAX"
		| "ICH.C.2.r.2.7.LENGTH.MAX"
		| "ICH.C.2.r.3.LENGTH.MAX"
		| "ICH.C.2.r.4.LENGTH.MAX"
		| "ICH.C.2.r.5.LENGTH.MAX"
		| "ICH.C.3.1.LENGTH.MAX"
		| "ICH.C.3.2.LENGTH.MAX"
		| "ICH.C.3.3.1.LENGTH.MAX"
		| "ICH.C.3.3.2.LENGTH.MAX"
		| "ICH.C.3.3.3.LENGTH.MAX"
		| "ICH.C.3.3.4.LENGTH.MAX"
		| "ICH.C.3.3.5.LENGTH.MAX"
		| "ICH.C.3.4.1.LENGTH.MAX"
		| "ICH.C.3.4.2.LENGTH.MAX"
		| "ICH.C.3.4.3.LENGTH.MAX"
		| "ICH.C.3.4.4.LENGTH.MAX"
		| "ICH.C.3.4.5.LENGTH.MAX"
		| "ICH.C.3.4.6.LENGTH.MAX"
		| "ICH.C.3.4.7.LENGTH.MAX"
		| "ICH.C.3.4.8.LENGTH.MAX"
		| "ICH.C.4.r.1.LENGTH.MAX"
		| "ICH.C.5.1.r.1.LENGTH.MAX"
		| "ICH.C.5.1.r.2.LENGTH.MAX"
		| "ICH.C.5.2.LENGTH.MAX"
		| "ICH.C.5.3.LENGTH.MAX"
		| "ICH.C.5.4.LENGTH.MAX"
		| "ICH.D.1.LENGTH.MAX"
		| "ICH.D.1.1.1.LENGTH.MAX"
		| "ICH.D.1.1.2.LENGTH.MAX"
		| "ICH.D.1.1.3.LENGTH.MAX"
		| "ICH.D.1.1.4.LENGTH.MAX"
		| "ICH.D.2.2a.LENGTH.MAX"
		| "ICH.D.2.2b.LENGTH.MAX"
		| "ICH.D.2.2.1a.LENGTH.MAX"
		| "ICH.D.2.2.1b.LENGTH.MAX"
		| "ICH.D.2.3.LENGTH.MAX"
		| "ICH.D.3.LENGTH.MAX"
		| "ICH.D.4.LENGTH.MAX"
		| "ICH.D.5.LENGTH.MAX"
		| "ICH.D.7.1.r.1a.LENGTH.MAX"
		| "ICH.D.7.1.r.1b.LENGTH.MAX"
		| "ICH.D.7.1.r.5.LENGTH.MAX"
		| "ICH.D.7.2.LENGTH.MAX"
		| "ICH.D.8.r.1.LENGTH.MAX"
		| "ICH.D.8.r.2a.LENGTH.MAX"
		| "ICH.D.8.r.2b.LENGTH.MAX"
		| "ICH.D.8.r.3a.LENGTH.MAX"
		| "ICH.D.8.r.3b.LENGTH.MAX"
		| "ICH.D.8.r.6a.LENGTH.MAX"
		| "ICH.D.8.r.6b.LENGTH.MAX"
		| "ICH.D.8.r.7a.LENGTH.MAX"
		| "ICH.D.8.r.7b.LENGTH.MAX"
		| "ICH.D.9.2.r.1a.LENGTH.MAX"
		| "ICH.D.9.2.r.1b.LENGTH.MAX"
		| "ICH.D.9.2.r.2.LENGTH.MAX"
		| "ICH.D.9.4.r.1a.LENGTH.MAX"
		| "ICH.D.9.4.r.1b.LENGTH.MAX"
		| "ICH.D.9.4.r.2.LENGTH.MAX"
		| "ICH.D.10.1.LENGTH.MAX"
		| "ICH.D.10.2.2a.LENGTH.MAX"
		| "ICH.D.10.2.2b.LENGTH.MAX"
		| "ICH.D.10.4.LENGTH.MAX"
		| "ICH.D.10.5.LENGTH.MAX"
		| "ICH.D.10.6.LENGTH.MAX"
		| "ICH.D.10.7.1.r.1a.LENGTH.MAX"
		| "ICH.D.10.7.1.r.1b.LENGTH.MAX"
		| "ICH.D.10.7.1.r.5.LENGTH.MAX"
		| "ICH.D.10.7.2.LENGTH.MAX"
		| "ICH.D.10.8.r.1.LENGTH.MAX"
		| "ICH.D.10.8.r.2a.LENGTH.MAX"
		| "ICH.D.10.8.r.2b.LENGTH.MAX"
		| "ICH.D.10.8.r.3a.LENGTH.MAX"
		| "ICH.D.10.8.r.3b.LENGTH.MAX"
		| "ICH.D.10.8.r.6a.LENGTH.MAX"
		| "ICH.D.10.8.r.6b.LENGTH.MAX"
		| "ICH.D.10.8.r.7a.LENGTH.MAX"
		| "ICH.D.10.8.r.7b.LENGTH.MAX"
		| "ICH.E.i.1.1a.LENGTH.MAX"
		| "ICH.E.i.1.1b.LENGTH.MAX"
		| "ICH.E.i.1.2.LENGTH.MAX"
		| "ICH.E.i.2.1a.LENGTH.MAX"
		| "ICH.E.i.2.1b.LENGTH.MAX"
		| "ICH.E.i.3.1.LENGTH.MAX"
		| "ICH.E.i.6a.LENGTH.MAX"
		| "ICH.E.i.6b.LENGTH.MAX"
		| "ICH.E.i.7.LENGTH.MAX"
		| "ICH.E.i.9.LENGTH.MAX"
		| "ICH.F.r.2.1.LENGTH.MAX"
		| "ICH.F.r.2.2a.LENGTH.MAX"
		| "ICH.F.r.2.2b.LENGTH.MAX"
		| "ICH.F.r.3.1.LENGTH.MAX"
		| "ICH.F.r.3.2.LENGTH.MAX"
		| "ICH.F.r.3.3.LENGTH.MAX"
		| "ICH.F.r.3.4.LENGTH.MAX"
		| "ICH.F.r.4.LENGTH.MAX"
		| "ICH.F.r.5.LENGTH.MAX"
		| "ICH.F.r.6.LENGTH.MAX"
		| "ICH.G.k.1.LENGTH.MAX"
		| "ICH.G.k.2.1.1a.LENGTH.MAX"
		| "ICH.G.k.2.1.1b.LENGTH.MAX"
		| "ICH.G.k.2.1.2a.LENGTH.MAX"
		| "ICH.G.k.2.1.2b.LENGTH.MAX"
		| "ICH.G.k.2.2.LENGTH.MAX"
		| "ICH.G.k.2.3.r.1.LENGTH.MAX"
		| "ICH.G.k.2.3.r.2a.LENGTH.MAX"
		| "ICH.G.k.2.3.r.2b.LENGTH.MAX"
		| "ICH.G.k.2.3.r.3a.LENGTH.MAX"
		| "ICH.G.k.2.3.r.3b.LENGTH.MAX"
		| "ICH.G.k.2.4.LENGTH.MAX"
		| "ICH.G.k.3.1.LENGTH.MAX"
		| "ICH.G.k.3.2.LENGTH.MAX"
		| "ICH.G.k.3.3.LENGTH.MAX"
		| "ICH.G.k.4.r.1a.LENGTH.MAX"
		| "ICH.G.k.4.r.1b.LENGTH.MAX"
		| "ICH.G.k.4.r.2.LENGTH.MAX"
		| "ICH.G.k.4.r.3.LENGTH.MAX"
		| "ICH.G.k.4.r.6a.LENGTH.MAX"
		| "ICH.G.k.4.r.6b.LENGTH.MAX"
		| "ICH.G.k.4.r.7.LENGTH.MAX"
		| "ICH.G.k.4.r.8.LENGTH.MAX"
		| "ICH.G.k.4.r.9.1.LENGTH.MAX"
		| "ICH.G.k.4.r.9.2a.LENGTH.MAX"
		| "ICH.G.k.4.r.9.2b.LENGTH.MAX"
		| "ICH.G.k.4.r.10.1.LENGTH.MAX"
		| "ICH.G.k.4.r.10.2a.LENGTH.MAX"
		| "ICH.G.k.4.r.10.2b.LENGTH.MAX"
		| "ICH.G.k.4.r.11.1.LENGTH.MAX"
		| "ICH.G.k.4.r.11.2a.LENGTH.MAX"
		| "ICH.G.k.4.r.11.2b.LENGTH.MAX"
		| "ICH.G.k.5a.LENGTH.MAX"
		| "ICH.G.k.5b.LENGTH.MAX"
		| "ICH.G.k.6a.LENGTH.MAX"
		| "ICH.G.k.6b.LENGTH.MAX"
		| "ICH.G.k.7.r.1.LENGTH.MAX"
		| "ICH.G.k.7.r.2a.LENGTH.MAX"
		| "ICH.G.k.7.r.2b.LENGTH.MAX"
		| "ICH.G.k.8.LENGTH.MAX"
		| "ICH.G.k.9.i.2.r.1.LENGTH.MAX"
		| "ICH.G.k.9.i.2.r.2.LENGTH.MAX"
		| "ICH.G.k.9.i.2.r.3.LENGTH.MAX"
		| "ICH.G.k.9.i.3.1a.LENGTH.MAX"
		| "ICH.G.k.9.i.3.1b.LENGTH.MAX"
		| "ICH.G.k.9.i.3.2a.LENGTH.MAX"
		| "ICH.G.k.9.i.3.2b.LENGTH.MAX"
		| "ICH.G.k.9.i.4.LENGTH.MAX"
		| "ICH.G.k.10.r.LENGTH.MAX"
		| "ICH.G.k.11.LENGTH.MAX"
		| "ICH.H.1.LENGTH.MAX"
		| "ICH.H.2.LENGTH.MAX"
		| "ICH.H.3.r.1a.LENGTH.MAX"
		| "ICH.H.3.r.1b.LENGTH.MAX"
		| "ICH.H.4.LENGTH.MAX"
		| "ICH.H.5.r.1a.LENGTH.MAX"
		| "ICH.H.5.r.1b.LENGTH.MAX"
		| "ICH.N.1.1.LENGTH.MAX"
		| "ICH.N.1.2.LENGTH.MAX"
		| "ICH.N.1.3.LENGTH.MAX"
		| "ICH.N.1.4.LENGTH.MAX"
		| "ICH.N.2.r.1.LENGTH.MAX"
		| "ICH.N.2.r.2.LENGTH.MAX"
		| "ICH.N.2.r.3.LENGTH.MAX" => PHASES_CASE_VALIDATE,
		_ => PHASES_METADATA_ONLY,
	}
}

fn to_canonical_allowed_value_rule<'a>(
	rule: &'a AllowedValueRuleMetadata,
) -> CanonicalRule<'a> {
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: section_for_rule_code(rule.code),
		blocking: true,
		category: RuleCategory::CaseBusiness,
		phases: phases_for_allowed_value_rule(rule.code),
		severity: RuleSeverity::Blocking,
		message: "Dictionary allowed values constraint.",
		condition: RuleCondition::Always,
	}
}

fn phases_for_allowed_value_rule(code: &str) -> &'static [ValidationPhase] {
	match code {
		"ICH.N.1.1.ALLOWED.VALUE"
		| "ICH.N.2.r.4.ALLOWED.VALUE"
		| "ICH.C.1.2.ALLOWED.VALUE"
		| "ICH.C.1.3.ALLOWED.VALUE"
		| "ICH.C.1.6.1.r.2.ALLOWED.VALUE"
		| "ICH.C.1.8.1.ALLOWED.VALUE"
		| "ICH.C.1.8.2.ALLOWED.VALUE"
		| "ICH.C.1.9.1.ALLOWED.VALUE"
		| "ICH.C.1.9.1.r.2.ALLOWED.VALUE"
		| "ICH.C.1.11.1.ALLOWED.VALUE"
		| "ICH.C.4.r.2.ALLOWED.VALUE"
		| "ICH.C.2.r.4.ALLOWED.VALUE"
		| "ICH.C.2.r.5.ALLOWED.VALUE"
		| "ICH.C.3.1.ALLOWED.VALUE"
		| "ICH.C.5.4.ALLOWED.VALUE"
		| "ICH.D.2.3.ALLOWED.VALUE"
		| "ICH.D.5.ALLOWED.VALUE"
		| "ICH.D.7.1.r.6.ALLOWED.VALUE"
		| "ICH.D.7.3.ALLOWED.VALUE"
		| "ICH.D.10.6.ALLOWED.VALUE"
		| "ICH.D.7.1.r.1a.ALLOWED.VALUE"
		| "ICH.D.7.1.r.1b.ALLOWED.VALUE"
		| "ICH.D.8.r.2b.ALLOWED.VALUE"
		| "ICH.D.8.r.3b.ALLOWED.VALUE"
		| "ICH.D.8.r.6a.ALLOWED.VALUE"
		| "ICH.D.8.r.6b.ALLOWED.VALUE"
		| "ICH.D.8.r.7a.ALLOWED.VALUE"
		| "ICH.D.8.r.7b.ALLOWED.VALUE"
		| "ICH.D.9.2.r.1a.ALLOWED.VALUE"
		| "ICH.D.9.2.r.1b.ALLOWED.VALUE"
		| "ICH.D.9.4.r.1a.ALLOWED.VALUE"
		| "ICH.D.9.4.r.1b.ALLOWED.VALUE"
		| "ICH.D.10.7.1.r.1a.ALLOWED.VALUE"
		| "ICH.D.10.7.1.r.1b.ALLOWED.VALUE"
		| "ICH.D.10.8.r.6a.ALLOWED.VALUE"
		| "ICH.D.10.8.r.6b.ALLOWED.VALUE"
		| "ICH.D.10.8.r.7a.ALLOWED.VALUE"
		| "ICH.D.10.8.r.7b.ALLOWED.VALUE"
		| "ICH.D.10.8.r.2b.ALLOWED.VALUE"
		| "ICH.D.10.8.r.3b.ALLOWED.VALUE"
		| "ICH.E.i.1.1b.ALLOWED.VALUE"
		| "ICH.E.i.2.1a.ALLOWED.VALUE"
		| "ICH.E.i.2.1b.ALLOWED.VALUE"
		| "ICH.E.i.3.2a.ALLOWED.VALUE"
		| "ICH.E.i.3.2b.ALLOWED.VALUE"
		| "ICH.E.i.3.2c.ALLOWED.VALUE"
		| "ICH.E.i.3.2d.ALLOWED.VALUE"
		| "ICH.E.i.3.2e.ALLOWED.VALUE"
		| "ICH.E.i.3.2f.ALLOWED.VALUE"
		| "ICH.E.i.7.ALLOWED.VALUE"
		| "ICH.F.r.2.2a.ALLOWED.VALUE"
		| "ICH.F.r.2.2b.ALLOWED.VALUE"
		| "ICH.F.r.3.3.ALLOWED.VALUE"
		| "ICH.F.r.3.1.ALLOWED.VALUE"
		| "ICH.F.r.3.2.ALLOWED.VALUE"
		| "ICH.G.k.1.ALLOWED.VALUE"
		| "ICH.G.k.2.5.ALLOWED.VALUE"
		| "ICH.G.k.8.ALLOWED.VALUE"
		| "ICH.G.k.9.i.4.ALLOWED.VALUE"
		| "ICH.G.k.10.r.ALLOWED.VALUE" => PHASES_CASE_VALIDATE,
		"ICH.G.k.2.1.1b.ALLOWED.VALUE"
		| "ICH.G.k.2.1.2b.ALLOWED.VALUE"
		| "ICH.G.k.2.3.r.2b.ALLOWED.VALUE"
		| "ICH.G.k.2.3.r.3b.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2b.ALLOWED.VALUE"
		| "ICH.H.3.r.1a.ALLOWED.VALUE"
		| "ICH.H.3.r.1b.ALLOWED.VALUE"
		| "ICH.H.5.r.1b.ALLOWED.VALUE" => PHASES_CASE_VALIDATE,
		_ => PHASES_METADATA_ONLY,
	}
}

fn to_canonical_vocabulary_rule<'a>(
	rule: &'a VocabularyRuleMetadata,
) -> CanonicalRule<'a> {
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: section_for_rule_code(rule.code),
		blocking: true,
		category: RuleCategory::CaseBusiness,
		phases: phases_for_vocabulary_rule(rule.code),
		severity: RuleSeverity::Blocking,
		message: "Dictionary vocabulary constraint.",
		condition: RuleCondition::Always,
	}
}

fn to_canonical_vocabulary_variant<'a>(
	rule: &'a VocabularyVariantMetadata,
) -> CanonicalRule<'a> {
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: section_for_rule_code(rule.code),
		blocking: true,
		category: RuleCategory::CaseBusiness,
		phases: PHASES_CASE_VALIDATE,
		severity: RuleSeverity::Blocking,
		message: "Dictionary receiver-specific vocabulary constraint.",
		condition: RuleCondition::Always,
	}
}

fn phases_for_vocabulary_rule(code: &str) -> &'static [ValidationPhase] {
	match code {
		"ICH.C.2.r.3.VOCABULARY"
		| "ICH.C.3.4.5.VOCABULARY"
		| "ICH.C.5.1.r.2.VOCABULARY"
		| "ICH.D.7.1.r.1a.VOCABULARY"
		| "ICH.D.7.1.r.1b.VOCABULARY"
		| "ICH.D.8.r.6a.VOCABULARY"
		| "ICH.D.8.r.6b.VOCABULARY"
		| "ICH.D.8.r.7a.VOCABULARY"
		| "ICH.D.8.r.7b.VOCABULARY"
		| "ICH.D.9.2.r.1a.VOCABULARY"
		| "ICH.D.9.2.r.1b.VOCABULARY"
		| "ICH.D.9.4.r.1a.VOCABULARY"
		| "ICH.D.9.4.r.1b.VOCABULARY"
		| "ICH.D.10.7.1.r.1a.VOCABULARY"
		| "ICH.D.10.7.1.r.1b.VOCABULARY"
		| "ICH.D.10.8.r.6a.VOCABULARY"
		| "ICH.D.10.8.r.6b.VOCABULARY"
		| "ICH.D.10.8.r.7a.VOCABULARY"
		| "ICH.D.10.8.r.7b.VOCABULARY"
		| "ICH.E.i.2.1a.VOCABULARY"
		| "ICH.E.i.2.1b.VOCABULARY"
		| "ICH.E.i.9.VOCABULARY"
		| "ICH.F.r.2.2a.VOCABULARY"
		| "ICH.F.r.2.2b.VOCABULARY"
		| "ICH.G.k.2.4.VOCABULARY"
		| "ICH.G.k.3.2.VOCABULARY"
		| "ICH.G.k.7.r.2a.VOCABULARY"
		| "ICH.G.k.7.r.2b.VOCABULARY"
		| "ICH.H.3.r.1a.VOCABULARY"
		| "ICH.H.3.r.1b.VOCABULARY" => PHASES_CASE_VALIDATE,
		_ => PHASES_METADATA_ONLY,
	}
}

fn to_canonical_null_flavor_rule<'a>(
	rule: &'a NullFlavorRuleMetadata,
) -> CanonicalRule<'a> {
	CanonicalRule {
		code: rule.code,
		authority: rule.authority,
		section: section_for_rule_code(rule.code),
		blocking: true,
		category: RuleCategory::CaseBusiness,
		phases: PHASES_METADATA_ONLY,
		severity: RuleSeverity::Blocking,
		message: "Dictionary nullFlavor allowed set.",
		condition: RuleCondition::Always,
	}
}

fn section_for_rule_code(code: &str) -> &'static str {
	let data_code = code
		.strip_prefix("ICH.")
		.or_else(|| code.strip_prefix("FDA."))
		.or_else(|| code.strip_prefix("MFDS."))
		.unwrap_or(code);
	if data_code.starts_with("C.") || data_code.starts_with("N.") {
		return "case-identification";
	}
	if data_code.starts_with("D.") {
		return "patient";
	}
	if data_code.starts_with("E.") {
		return "reactions";
	}
	if data_code.starts_with("F.") {
		return "tests";
	}
	if data_code.starts_with("G.") {
		return "drugs";
	}
	if data_code.starts_with("H.") {
		return "narrative";
	}
	"case"
}

fn category_for_rule(rule: &ValidationRuleMetadata) -> RuleCategory {
	if is_xml_structure_rule(rule) {
		RuleCategory::XmlStructure
	} else {
		RuleCategory::CaseBusiness
	}
}

fn phases_for_rule(rule: &ValidationRuleMetadata) -> &'static [ValidationPhase] {
	if is_dictionary_required_metadata_only(rule.code) {
		return PHASES_METADATA_ONLY;
	}
	if is_xml_structure_rule(rule) {
		return PHASES_IMPORT_ONLY;
	}
	PHASES_CASE_VALIDATE
}

fn is_dictionary_required_metadata_only(code: &str) -> bool {
	let _ = code;
	false
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
		.or_else(|| {
			MAX_LENGTH_RULES
				.iter()
				.filter(|rule| rule.code == code)
				.map(to_canonical_max_length_rule)
				.find(|rule| rule_applies_in_phase(*rule, phase))
		})
		.or_else(|| {
			ALLOWED_VALUE_RULES
				.iter()
				.filter(|rule| rule.code == code)
				.map(to_canonical_allowed_value_rule)
				.find(|rule| rule_applies_in_phase(*rule, phase))
		})
		.or_else(|| {
			VOCABULARY_RULES
				.iter()
				.filter(|rule| rule.code == code)
				.map(to_canonical_vocabulary_rule)
				.find(|rule| rule_applies_in_phase(*rule, phase))
		})
		.or_else(|| {
			VOCABULARY_VARIANTS
				.iter()
				.filter(|rule| rule.code == code)
				.map(to_canonical_vocabulary_variant)
				.find(|rule| rule_applies_in_phase(*rule, phase))
		})
		.or_else(|| {
			NULL_FLAVOR_RULES
				.iter()
				.filter(|rule| rule.code == code)
				.map(to_canonical_null_flavor_rule)
				.find(|rule| rule_applies_in_phase(*rule, phase))
		})
}

pub fn find_canonical_rule(code: &str) -> Option<CanonicalRule<'static>> {
	find_canonical_rule_for_phase(code, ValidationPhase::CaseValidate)
		.or_else(|| {
			VALIDATION_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_rule)
		})
		.or_else(|| {
			MAX_LENGTH_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_max_length_rule)
		})
		.or_else(|| {
			ALLOWED_VALUE_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_allowed_value_rule)
		})
		.or_else(|| {
			VOCABULARY_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_vocabulary_rule)
		})
		.or_else(|| {
			VOCABULARY_VARIANTS
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_vocabulary_variant)
		})
		.or_else(|| {
			NULL_FLAVOR_RULES
				.iter()
				.find(|rule| rule.code == code)
				.map(to_canonical_null_flavor_rule)
		})
}

pub fn canonical_rules_for_authority(
	authority: RegulatoryAuthority,
) -> Vec<CanonicalRule<'static>> {
	let mut rules = VALIDATION_RULES
		.iter()
		.filter(|rule| {
			matches!(rule.authority, RegulatoryAuthority::Ich)
				|| rule.authority == authority
		})
		.map(to_canonical_rule)
		.collect::<Vec<_>>();
	rules.extend(
		MAX_LENGTH_RULES
			.iter()
			.filter(|rule| {
				matches!(rule.authority, RegulatoryAuthority::Ich)
					|| rule.authority == authority
			})
			.map(to_canonical_max_length_rule),
	);
	rules.extend(
		ALLOWED_VALUE_RULES
			.iter()
			.filter(|rule| {
				matches!(rule.authority, RegulatoryAuthority::Ich)
					|| rule.authority == authority
			})
			.map(to_canonical_allowed_value_rule),
	);
	rules.extend(
		VOCABULARY_RULES
			.iter()
			.filter(|rule| {
				matches!(rule.authority, RegulatoryAuthority::Ich)
					|| rule.authority == authority
			})
			.map(to_canonical_vocabulary_rule),
	);
	for variant in VOCABULARY_VARIANTS.iter().filter(|rule| {
		matches!(rule.authority, RegulatoryAuthority::Ich)
			|| rule.authority == authority
	}) {
		if !rules.iter().any(|rule| rule.code == variant.code) {
			rules.push(to_canonical_vocabulary_variant(variant));
		}
	}
	rules.extend(
		NULL_FLAVOR_RULES
			.iter()
			.filter(|rule| {
				matches!(rule.authority, RegulatoryAuthority::Ich)
					|| rule.authority == authority
			})
			.map(to_canonical_null_flavor_rule),
	);
	rules
}

pub fn canonical_rules_for_authority_phase(
	authority: RegulatoryAuthority,
	phase: ValidationPhase,
) -> Vec<CanonicalRule<'static>> {
	canonical_rules_for_authority(authority)
		.into_iter()
		.filter(|rule| rule_applies_in_phase(*rule, phase))
		.collect()
}

pub fn canonical_rules_all() -> Vec<CanonicalRule<'static>> {
	let mut rules = VALIDATION_RULES
		.iter()
		.map(to_canonical_rule)
		.chain(MAX_LENGTH_RULES.iter().map(to_canonical_max_length_rule))
		.chain(
			ALLOWED_VALUE_RULES
				.iter()
				.map(to_canonical_allowed_value_rule),
		)
		.chain(VOCABULARY_RULES.iter().map(to_canonical_vocabulary_rule))
		.chain(NULL_FLAVOR_RULES.iter().map(to_canonical_null_flavor_rule))
		.collect::<Vec<_>>();
	for variant in VOCABULARY_VARIANTS {
		if !rules.iter().any(|rule| rule.code == variant.code) {
			rules.push(to_canonical_vocabulary_variant(variant));
		}
	}
	rules
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

pub fn canonical_rules_version(authority: Option<RegulatoryAuthority>) -> String {
	let rules = if let Some(authority) = authority {
		canonical_rules_for_authority(authority)
	} else {
		canonical_rules_all()
	};

	let mut hash: u64 = 14695981039346656037;
	for rule in rules {
		hash = fnv1a_update(hash, rule.code.as_bytes());
		hash = fnv1a_update(hash, b"|");
		hash = fnv1a_update(hash, rule.authority.as_str().as_bytes());
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
		hash = fnv1a_update(hash, b";");
	}

	format!("{hash:016x}")
}

pub fn max_length_for_rule(code: &str) -> Option<usize> {
	MAX_LENGTH_RULES
		.iter()
		.find(|rule| rule.code == code)
		.map(|rule| rule.max_length)
}

pub fn allowed_values_source_hash_for_rule(code: &str) -> Option<u64> {
	ALLOWED_VALUE_RULES
		.iter()
		.find(|rule| rule.code == code)
		.map(|rule| rule.source_hash)
}

pub fn allowed_value_constraint_for_rule(
	code: &str,
) -> Option<&'static AllowedValueConstraint> {
	allowed_value_constraints().get(code)
}

pub fn allowed_value_enforcement_for_rule(
	code: &str,
) -> Option<ConstraintEnforcement> {
	allowed_value_constraint_for_rule(code)?.enforcement
}

pub fn allowed_value_code_for_vocabulary_rule(code: &str) -> Option<&'static str> {
	let prefix = code.strip_suffix(".VOCABULARY")?;
	ALLOWED_VALUE_RULES
		.iter()
		.find(|rule| rule.code.strip_suffix(".ALLOWED.VALUE") == Some(prefix))
		.map(|rule| rule.code)
}

pub fn representation_enforced_rule_codes() -> BTreeSet<&'static str> {
	ALLOWED_VALUE_RULES
		.iter()
		.filter_map(|rule| {
			(allowed_value_enforcement_for_rule(rule.code)
				== Some(ConstraintEnforcement::RepresentationEnforced))
			.then_some(rule.code)
		})
		.collect()
}

pub fn vocabulary_for_rule(code: &str) -> Option<&'static str> {
	VOCABULARY_RULES
		.iter()
		.find(|rule| rule.code == code)
		.map(|rule| rule.vocabulary)
}

pub fn vocabulary_variant_for_rule(
	code: &str,
	receiver: &str,
) -> Option<&'static VocabularyVariantMetadata> {
	VOCABULARY_VARIANTS.iter().find(|variant| {
		variant.code == code && variant.receiver.eq_ignore_ascii_case(receiver)
	})
}

pub fn null_flavors_source_hash_for_rule(code: &str) -> Option<u64> {
	NULL_FLAVOR_RULES
		.iter()
		.find(|rule| rule.code == code)
		.map(|rule| rule.source_hash)
}

pub fn null_flavors_for_rule(code: &str) -> Option<&'static [String]> {
	let element_code = code
		.strip_prefix("ICH.")?
		.strip_suffix(".NULLFLAVOR.ALLOWED")?;
	embedded_ich_dictionary()
		.entries
		.iter()
		.find(|entry| entry.code == element_code)
		.map(|entry| entry.null_flavors.as_slice())
}

pub fn is_rule_condition_satisfied(code: &str, facts: RuleFacts) -> bool {
	let Some(rule) = find_canonical_rule(code) else {
		return true;
	};
	match rule.condition {
		RuleCondition::Always => true,
		RuleCondition::IchDictionaryCondition => true,
		RuleCondition::IchCaseHistoryTrueMissingPriorIds => facts
			.ich_case_history_true_missing_prior_ids
			.unwrap_or(false),
		RuleCondition::IchMedicalHistoryMissingD72Text => {
			facts.ich_medical_history_missing_d72_text.unwrap_or(false)
		}
		RuleCondition::IchReportTypeIsStudy => {
			facts.ich_report_type_is_study.unwrap_or(false)
		}
		RuleCondition::IchNullificationCodePresent => {
			facts.ich_nullification_code_present.unwrap_or(false)
		}
		RuleCondition::IchSenderOrganizationRequired => {
			facts.ich_sender_organization_required.unwrap_or(false)
		}
		RuleCondition::IchAgeValuePresent => {
			facts.ich_age_value_present.unwrap_or(false)
		}
		RuleCondition::IchAgeUnitPresent => {
			facts.ich_age_unit_present.unwrap_or(false)
		}
		RuleCondition::IchGestationValuePresent => {
			facts.ich_gestation_value_present.unwrap_or(false)
		}
		RuleCondition::IchGestationUnitPresent => {
			facts.ich_gestation_unit_present.unwrap_or(false)
		}
		RuleCondition::IchDateOfDeathPresent => {
			facts.ich_date_of_death_present.unwrap_or(false)
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
		RuleCondition::MfdsSenderTypeIsHealthProfessional => facts
			.mfds_sender_type_is_health_professional
			.unwrap_or(false),
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
		Some(ValuePolicy::FdaBooleanStringOrNullFlavor) => {
			let value = value_code.map(str::trim).filter(|v| !v.is_empty());
			matches!(value, Some("false" | "true")) || null_flavor.is_some()
		}
		Some(ValuePolicy::FdaLocalCriteriaAllowedCode) => value_code
			.map(str::trim)
			.map(|code| matches!(code, "1" | "2" | "4" | "5" | "6"))
			.unwrap_or(false),
		Some(ValuePolicy::MfdsHealthProfessionalTypeKr1) => value_code
			.map(str::trim)
			.map(|code| matches!(code, "1" | "2" | "3" | "4"))
			.unwrap_or(false),
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

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::{BTreeMap, BTreeSet, HashSet};

	#[derive(Debug, serde::Deserialize)]
	struct Dictionary {
		entries: Vec<DictionaryEntry>,
	}

	#[derive(Debug, serde::Deserialize)]
	struct DictionaryEntry {
		code: String,
		kind: String,
		conformance: Option<String>,
		condition_text: Option<String>,
		data_type: Option<String>,
		max_length: Option<String>,
		allowed_values: Option<String>,
		allowed_value_constraint: Option<AllowedValueConstraint>,
		vocabulary: Option<serde_json::Value>,
		#[serde(default)]
		vocabulary_variants: Vec<DictionaryVocabularyVariant>,
		#[serde(default)]
		null_flavors: Vec<String>,
	}

	#[derive(Debug, serde::Deserialize)]
	struct DictionaryVocabularyVariant {
		receiver: String,
		vocabulary: String,
		vocabulary_scope: VocabularyScope,
	}

	fn dictionary_from_json(source: &str) -> Dictionary {
		serde_json::from_str(source).expect("dictionary should parse")
	}

	fn all_dictionary_entries() -> Vec<DictionaryEntry> {
		let mut entries = Vec::new();
		entries.extend(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			))
			.entries,
		);
		entries.extend(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			))
			.entries,
		);
		entries.extend(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			))
			.entries,
		);
		entries
	}

	fn required_dictionary_codes(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, Vec<String>)> {
		dictionary
			.entries
			.into_iter()
			.filter(|entry| {
				entry.kind == "element"
					&& matches!(
						entry.conformance.as_deref(),
						Some("mandatory" | "required" | "conditional_mandatory")
					)
			})
			.map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}{}", entry.code)
				};
				(format!("{code}.REQUIRED"), entry.null_flavors)
			})
			.collect()
	}

	fn ich_required_dictionary_codes() -> Vec<(String, Vec<String>)> {
		required_dictionary_codes(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH.",
		)
	}

	fn ich_required_dictionary_conformance_counts() -> BTreeMap<String, usize> {
		let dictionary = dictionary_from_json(include_str!(
			"../../../../registry/dictionary/ich-e2br3.json"
		));
		let mut counts = BTreeMap::new();
		for entry in dictionary.entries {
			if entry.kind == "element" {
				if let Some(conformance) = entry.conformance {
					if matches!(
						conformance.as_str(),
						"mandatory" | "required" | "conditional_mandatory"
					) {
						*counts.entry(conformance).or_insert(0) += 1;
					}
				}
			}
		}
		counts
	}

	fn ich_conditional_dictionary_codes() -> Vec<(String, String)> {
		dictionary_from_json(include_str!(
			"../../../../registry/dictionary/ich-e2br3.json"
		))
		.entries
		.into_iter()
		.filter(|entry| {
			entry.kind == "element"
				&& entry.conformance.as_deref() == Some("conditional_mandatory")
		})
		.map(|entry| {
			(
				format!("ICH.{}.REQUIRED", entry.code),
				entry.condition_text.expect(
					"ICH conditional mandatory entries carry condition_text",
				),
			)
		})
		.collect()
	}

	fn ich_non_conditional_required_dictionary_codes() -> Vec<String> {
		dictionary_from_json(include_str!(
			"../../../../registry/dictionary/ich-e2br3.json"
		))
		.entries
		.into_iter()
		.filter(|entry| {
			entry.kind == "element"
				&& matches!(
					entry.conformance.as_deref(),
					Some("mandatory" | "required")
				)
		})
		.map(|entry| format!("ICH.{}.REQUIRED", entry.code))
		.collect()
	}

	fn ich_date_time_dictionary_codes() -> Vec<String> {
		let mut codes = dictionary_from_json(include_str!(
			"../../../../registry/dictionary/ich-e2br3.json"
		))
		.entries
		.into_iter()
		.filter(|entry| {
			entry.kind == "element"
				&& entry.data_type.as_deref() == Some("Date/Time")
		})
		.map(|entry| format!("ICH.{}", entry.code))
		.collect::<Vec<_>>();
		codes.sort();
		codes
	}

	fn parse_dictionary_max_length(value: &str) -> usize {
		let value = value.trim();
		if let Some(r3_pos) = value.find("E2B(R3)") {
			let prefix = value[..r3_pos].trim_end();
			let digits = prefix
				.chars()
				.rev()
				.skip_while(|ch| ch.is_whitespace() || *ch == '*')
				.take_while(|ch| ch.is_ascii_digit())
				.collect::<String>()
				.chars()
				.rev()
				.collect::<String>();
			return digits
				.parse()
				.expect("R3 max_length should contain numeric length");
		}
		value.parse().expect("max_length should be numeric")
	}

	fn max_length_dictionary_rules(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, usize)> {
		dictionary
			.entries
			.into_iter()
			.filter(|entry| {
				entry.kind == "element"
					&& entry
						.max_length
						.as_deref()
						.is_some_and(|value| !value.trim().is_empty())
			})
			.map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}.{}", entry.code)
				};
				(
					format!("{code}.LENGTH.MAX"),
					parse_dictionary_max_length(
						entry.max_length.as_deref().unwrap_or_default(),
					),
				)
			})
			.collect()
	}

	fn dictionary_text_hash(value: &str) -> u64 {
		let mut hash: u64 = 14695981039346656037;
		hash = fnv1a_update(hash, value.trim().as_bytes());
		hash
	}

	fn allowed_values_dictionary_rules(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, u64)> {
		dictionary
			.entries
			.into_iter()
			.filter(|entry| {
				entry.kind == "element"
					&& entry
						.allowed_values
						.as_deref()
						.is_some_and(|value| !value.trim().is_empty())
			})
			.map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}.{}", entry.code)
				};
				(
					format!("{code}.ALLOWED.VALUE"),
					dictionary_text_hash(
						entry.allowed_values.as_deref().unwrap_or_default(),
					),
				)
			})
			.collect()
	}

	fn allowed_value_constraint_dictionary_rules(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, AllowedValueConstraint)> {
		dictionary
			.entries
			.into_iter()
			.filter_map(|entry| {
				entry.allowed_value_constraint.map(|constraint| {
					let code = if entry.code.starts_with(authority_prefix) {
						entry.code
					} else {
						format!("{authority_prefix}.{}", entry.code)
					};
					(format!("{code}.ALLOWED.VALUE"), constraint)
				})
			})
			.collect()
	}

	fn vocabulary_dictionary_rules(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, String)> {
		dictionary
			.entries
			.into_iter()
			.filter(|entry| entry.kind == "element" && entry.vocabulary.is_some())
			.map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}.{}", entry.code)
				};
				let vocabulary = entry
					.vocabulary
					.expect("filtered vocabulary entry")
					.as_str()
					.map(str::to_string)
					.unwrap_or_else(|| "non_string_vocabulary".to_string());
				(format!("{code}.VOCABULARY"), vocabulary)
			})
			.collect()
	}

	fn null_flavors_dictionary_rules(
		dictionary: Dictionary,
		authority_prefix: &str,
	) -> Vec<(String, u64)> {
		dictionary
			.entries
			.into_iter()
			.filter(|entry| {
				entry.kind == "element" && !entry.null_flavors.is_empty()
			})
			.map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}.{}", entry.code)
				};
				(
					format!("{code}.NULLFLAVOR.ALLOWED"),
					dictionary_text_hash(&entry.null_flavors.join(",")),
				)
			})
			.collect()
	}

	fn all_max_length_dictionary_rules() -> Vec<(String, usize)> {
		let mut rules = Vec::new();
		rules.extend(max_length_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH",
		));
		rules.extend(max_length_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			)),
			"FDA",
		));
		rules.extend(max_length_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			)),
			"MFDS",
		));
		rules
	}

	fn all_null_flavors_dictionary_rules() -> Vec<(String, u64)> {
		let mut rules = Vec::new();
		rules.extend(null_flavors_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH",
		));
		rules.extend(null_flavors_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			)),
			"FDA",
		));
		rules.extend(null_flavors_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			)),
			"MFDS",
		));
		rules
	}

	fn all_allowed_values_dictionary_rules() -> Vec<(String, u64)> {
		let mut rules = Vec::new();
		rules.extend(allowed_values_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH",
		));
		rules.extend(allowed_values_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			)),
			"FDA",
		));
		rules.extend(allowed_values_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			)),
			"MFDS",
		));
		rules
	}

	fn all_vocabulary_dictionary_rules() -> Vec<(String, String)> {
		let mut rules = Vec::new();
		rules.extend(vocabulary_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH",
		));
		rules.extend(vocabulary_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			)),
			"FDA",
		));
		rules.extend(vocabulary_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			)),
			"MFDS",
		));
		rules
	}

	fn all_vocabulary_dictionary_variants(
	) -> Vec<(String, String, String, VocabularyScope)> {
		let mut variants = Vec::new();
		for (dictionary, authority_prefix) in [
			(
				dictionary_from_json(include_str!(
					"../../../../registry/dictionary/ich-e2br3.json"
				)),
				"ICH",
			),
			(
				dictionary_from_json(include_str!(
					"../../../../registry/dictionary/fda-regional.json"
				)),
				"FDA",
			),
			(
				dictionary_from_json(include_str!(
					"../../../../registry/dictionary/mfds-regional.json"
				)),
				"MFDS",
			),
		] {
			variants.extend(dictionary.entries.into_iter().flat_map(|entry| {
				let code = if entry.code.starts_with(authority_prefix) {
					entry.code
				} else {
					format!("{authority_prefix}.{}", entry.code)
				};
				entry.vocabulary_variants.into_iter().map(move |variant| {
					(
						format!("{code}.VOCABULARY"),
						variant.receiver,
						variant.vocabulary,
						variant.vocabulary_scope,
					)
				})
			}));
		}
		variants
	}

	const CLASSIFIED_ICH_DATE_TIME_FUTURE_RULES: &[&str] = &[
		"ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
		"ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
		"ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
		"ICH.D.6.FUTURE_DATE.FORBIDDEN",
		"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
		"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		"ICH.N.1.5.FUTURE_DATE.FORBIDDEN",
		"ICH.N.2.r.4.FUTURE_DATE.FORBIDDEN",
	];

	const CLASSIFIED_ICH_DATE_TIME_FUTURE_RULE_COVERAGE: &[&str] = &[
		"ICH.C.1.2",
		"ICH.C.1.4",
		"ICH.C.1.5",
		"ICH.D.10.2.1",
		"ICH.D.10.3",
		"ICH.D.10.7.1.r.2",
		"ICH.D.10.7.1.r.4",
		"ICH.D.10.8.r.4",
		"ICH.D.10.8.r.5",
		"ICH.D.2.1",
		"ICH.D.6",
		"ICH.D.7.1.r.2",
		"ICH.D.7.1.r.4",
		"ICH.D.8.r.4",
		"ICH.D.8.r.5",
		"ICH.D.9.1",
		"ICH.E.i.4",
		"ICH.E.i.5",
		"ICH.F.r.1",
		"ICH.G.k.4.r.4",
		"ICH.G.k.4.r.5",
		"ICH.N.1.5",
		"ICH.N.2.r.4",
	];

	const CLASSIFIED_ICH_DATE_TIME_WITHOUT_FUTURE_RULE: &[&str] = &[];

	const CLASSIFIED_ICH_CATALOG_REQUIRED_EXTRAS: &[&str] = &[
		"ICH.C.1.REQUIRED",
		"ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED",
		"ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED",
		"ICH.C.2.r.2.1.REQUIRED",
		"ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED",
		"ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED",
		"ICH.C.5.3.REQUIRED",
		"ICH.C.5.TITLE.NULLFLAVOR.REQUIRED",
		"ICH.D.1.1.4.REQUIRED",
		"ICH.D.10.6.REQUIRED",
		"ICH.D.10.8.r.2a.REQUIRED",
		"ICH.D.10.8.r.3a.REQUIRED",
		"ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED",
		"ICH.D.9.2.r.2.REQUIRED",
		"ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
		"ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED",
		"ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED",
		"ICH.E.i.1.1a.REQUIRED",
		"ICH.E.i.3.2.CRITERIA.REQUIRED",
		"ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		"ICH.F.r.2.1.REQUIRED",
		"ICH.F.r.2.REQUIRED",
		"ICH.G.k.2.3.r.1.REQUIRED",
		"ICH.G.k.2.3.r.2a.REQUIRED",
		"ICH.G.k.2.3.r.3b.REQUIRED",
		"ICH.G.k.4.r.10.2a.REQUIRED",
		"ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
		"ICH.G.k.4.r.11.2a.REQUIRED",
		"ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED",
		"ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		"ICH.G.k.4.r.9.2a.REQUIRED",
		"ICH.N.REQUIRED",
		"ICH.XML.BL.NULLFLAVOR.REQUIRED",
		"ICH.XML.CODE.NULLFLAVOR.REQUIRED",
		"ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED",
		"ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED",
		"ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED",
		"ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED",
		"ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED",
		"ICH.XML.MEDDRA.VERSION.REQUIRED",
		"ICH.XML.PERIOD.VALUE_UNIT.REQUIRED",
		"ICH.XML.PIVL_TS.PERIOD.REQUIRED",
		"ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED",
		"ICH.XML.ROOT.ITSVERSION.REQUIRED",
		"ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
		"ICH.XML.SXPR_TS.COMP.REQUIRED",
		"ICH.XML.TELECOM.FORMAT.REQUIRED",
		"ICH.XML.TELECOM.NULLFLAVOR.REQUIRED",
		"ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED",
		"ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED",
		"ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED",
		"ICH.XML.TEXT.NULLFLAVOR.REQUIRED",
	];

	fn fda_required_dictionary_codes() -> Vec<(String, Vec<String>)> {
		required_dictionary_codes(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/fda-regional.json"
			)),
			"FDA.",
		)
	}

	fn mfds_required_dictionary_codes() -> Vec<(String, Vec<String>)> {
		required_dictionary_codes(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/mfds-regional.json"
			)),
			"MFDS.",
		)
	}

	fn value_policy_for_code(code: &str) -> Option<ValuePolicy> {
		VALUE_POLICY_BINDINGS
			.iter()
			.find(|binding| binding.code == code)
			.map(|binding| binding.policy)
	}

	#[test]
	fn ich_dictionary_required_candidate_inventory_is_stable() {
		let counts = ich_required_dictionary_conformance_counts();
		assert_eq!(
			counts,
			BTreeMap::from([
				("conditional_mandatory".to_string(), 62),
				("mandatory".to_string(), 32),
				("required".to_string(), 2),
			]),
			"ICH dictionary required candidate inventory changed"
		);
		assert_eq!(
			ich_required_dictionary_codes().len(),
			96,
			"ICH dictionary required candidate count changed"
		);
	}

	#[test]
	fn ich_date_time_dictionary_inventory_is_stable() {
		let expected = CLASSIFIED_ICH_DATE_TIME_FUTURE_RULE_COVERAGE
			.iter()
			.chain(CLASSIFIED_ICH_DATE_TIME_WITHOUT_FUTURE_RULE.iter())
			.map(|code| (*code).to_string())
			.collect::<BTreeSet<_>>();
		let actual = ich_date_time_dictionary_codes()
			.into_iter()
			.collect::<BTreeSet<_>>();

		assert_eq!(
			actual, expected,
			"ICH Date/Time dictionary inventory changed; review FUTURE_DATE coverage classification"
		);
	}

	#[test]
	fn dictionary_value_constraint_candidate_inventory_is_stable() {
		let entries = all_dictionary_entries();
		let mut counts = BTreeMap::new();
		counts.insert("allowed_values", 0usize);
		counts.insert("date_time", 0usize);
		counts.insert("max_length", 0usize);
		counts.insert("null_flavors", 0usize);
		counts.insert("vocabulary", 0usize);

		for entry in entries.iter().filter(|entry| entry.kind == "element") {
			if entry
				.allowed_values
				.as_deref()
				.is_some_and(|value| !value.trim().is_empty())
			{
				*counts.get_mut("allowed_values").unwrap() += 1;
			}
			if entry.data_type.as_deref() == Some("Date/Time") {
				*counts.get_mut("date_time").unwrap() += 1;
			}
			if entry
				.max_length
				.as_deref()
				.is_some_and(|value| !value.trim().is_empty())
			{
				*counts.get_mut("max_length").unwrap() += 1;
			}
			if !entry.null_flavors.is_empty() {
				*counts.get_mut("null_flavors").unwrap() += 1;
			}
			if entry.vocabulary.is_some() || !entry.vocabulary_variants.is_empty() {
				*counts.get_mut("vocabulary").unwrap() += 1;
			}
		}

		assert_eq!(
			counts,
			BTreeMap::from([
				("allowed_values", 261),
				("date_time", 23),
				("max_length", 218),
				("null_flavors", 72),
				("vocabulary", 59),
			]),
			"dictionary value-constraint candidate inventory changed"
		);
	}

	#[test]
	fn catalog_value_constraint_rule_inventory_is_stable() {
		let catalog_codes = canonical_rules_all()
			.into_iter()
			.map(|rule| rule.code)
			.collect::<BTreeSet<_>>();
		let counts = BTreeMap::from([
			(
				"allowed",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".ALLOWED."))
					.count(),
			),
			(
				"exclusive",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".EXCLUSIVE"))
					.count(),
			),
			(
				"format",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".FORMAT."))
					.count(),
			),
			(
				"future_date",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".FUTURE_DATE."))
					.count(),
			),
			(
				"length",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".LENGTH."))
					.count(),
			),
			(
				"null_flavor",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".NULLFLAVOR."))
					.count(),
			),
			(
				"vocabulary",
				catalog_codes
					.iter()
					.filter(|code| code.contains(".VOCABULARY"))
					.count(),
			),
		]);

		assert_eq!(
			counts,
			BTreeMap::from([
				("allowed", 261),
				("exclusive", 2),
				("format", 3),
				("future_date", 17),
				("length", 218),
				("null_flavor", 104),
				("vocabulary", 59),
			]),
			"catalog value-constraint rule inventory changed"
		);
	}

	#[test]
	fn dictionary_max_length_rules_are_catalog_backed() {
		let dictionary_rules = all_max_length_dictionary_rules();
		assert_eq!(
			dictionary_rules.len(),
			218,
			"dictionary max_length rule count changed"
		);

		let missing = dictionary_rules
			.iter()
			.filter(|(code, _)| find_canonical_rule(code).is_none())
			.collect::<Vec<_>>();
		assert!(
			missing.is_empty(),
			"dictionary max_length rules missing from catalog: {missing:?}"
		);

		let mismatched = dictionary_rules
			.iter()
			.filter(|(code, expected)| max_length_for_rule(code) != Some(*expected))
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary max_length values differ from catalog: {mismatched:?}"
		);
	}

	#[test]
	fn dictionary_allowed_values_rules_are_catalog_backed() {
		let dictionary_rules = all_allowed_values_dictionary_rules();
		assert_eq!(
			dictionary_rules.len(),
			261,
			"dictionary allowed_values rule count changed"
		);

		let missing = dictionary_rules
			.iter()
			.filter(|(code, _)| find_canonical_rule(code).is_none())
			.collect::<Vec<_>>();
		assert!(
			missing.is_empty(),
			"dictionary allowed_values rules missing from catalog: {missing:?}"
		);

		let mismatched = dictionary_rules
			.iter()
			.filter(|(code, expected)| {
				allowed_values_source_hash_for_rule(code) != Some(*expected)
			})
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary allowed_values source hashes differ from catalog: {mismatched:?}"
		);
	}

	#[test]
	fn dictionary_allowed_value_constraints_match_catalog_exactly() {
		let dictionary_rules = allowed_value_constraint_dictionary_rules(
			dictionary_from_json(include_str!(
				"../../../../registry/dictionary/ich-e2br3.json"
			)),
			"ICH",
		);
		assert_eq!(
			dictionary_rules.len(),
			223,
			"ICH allowed_value_constraint count changed"
		);

		let mismatched = dictionary_rules
			.iter()
			.filter(|(code, expected)| {
				allowed_value_constraint_for_rule(code) != Some(expected)
			})
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary allowed_value_constraint values differ from catalog: {mismatched:?}"
		);

		let dictionary_codes = dictionary_rules
			.iter()
			.map(|(code, _)| code.as_str())
			.collect::<BTreeSet<_>>();
		let catalog_codes = allowed_value_constraints()
			.keys()
			.map(String::as_str)
			.collect::<BTreeSet<_>>();
		assert_eq!(
			dictionary_codes, catalog_codes,
			"catalog allowed_value_constraint codes must exactly match the dictionary"
		);
	}

	#[test]
	fn ich_machine_constraints_have_enforcement() {
		let machine = allowed_value_constraints()
			.values()
			.filter(|constraint| {
				constraint.kind != AllowedValueConstraintKind::Descriptive
			})
			.collect::<Vec<_>>();

		assert_eq!(machine.len(), 133);
		assert!(machine
			.iter()
			.all(|constraint| constraint.enforcement.is_some()));
	}

	#[test]
	fn ich_structured_allowed_value_target_baseline_is_exact() {
		let generated = ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES
			.iter()
			.copied()
			.collect::<BTreeSet<_>>();

		assert_eq!(generated.len(), 103);
		for (kind, count) in [
			(AllowedValueConstraintKind::Numeric, 40),
			(AllowedValueConstraintKind::Vocabulary, 26),
			(AllowedValueConstraintKind::Format, 23),
			(AllowedValueConstraintKind::Boolean, 7),
			(AllowedValueConstraintKind::TrueMarker, 6),
			(AllowedValueConstraintKind::CodeSet, 1),
		] {
			assert_eq!(
				generated
					.iter()
					.filter(|code| {
						allowed_value_constraint_for_rule(code).unwrap().kind == kind
					})
					.count(),
				count,
				"unexpected target count for {kind:?}"
			);
		}
	}

	#[test]
	fn dictionary_vocabulary_rules_are_catalog_backed() {
		let dictionary_rules = all_vocabulary_dictionary_rules();
		assert_eq!(
			dictionary_rules.len(),
			56,
			"dictionary vocabulary rule count changed"
		);

		let missing = dictionary_rules
			.iter()
			.filter(|(code, _)| find_canonical_rule(code).is_none())
			.collect::<Vec<_>>();
		assert!(
			missing.is_empty(),
			"dictionary vocabulary rules missing from catalog: {missing:?}"
		);

		let mismatched = dictionary_rules
			.iter()
			.filter(|(code, expected)| {
				vocabulary_for_rule(code) != Some(expected.as_str())
			})
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary vocabulary values differ from catalog: {mismatched:?}"
		);
	}

	#[test]
	fn dictionary_vocabulary_variants_match_catalog_exactly() {
		let dictionary_variants = all_vocabulary_dictionary_variants();
		assert_eq!(dictionary_variants.len(), 6);

		let mismatched = dictionary_variants
			.iter()
			.filter(|(code, receiver, vocabulary, scope)| {
				vocabulary_variant_for_rule(code, receiver)
					.map(|variant| (variant.vocabulary, variant.scope))
					!= Some((vocabulary.as_str(), *scope))
			})
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary vocabulary variants differ from catalog: {mismatched:?}"
		);
		assert_eq!(VOCABULARY_VARIANTS.len(), dictionary_variants.len());
	}

	#[test]
	fn dictionary_null_flavor_allowed_rules_are_catalog_backed() {
		let dictionary_rules = all_null_flavors_dictionary_rules();
		assert_eq!(
			dictionary_rules.len(),
			72,
			"dictionary null_flavors rule count changed"
		);

		let missing = dictionary_rules
			.iter()
			.filter(|(code, _)| find_canonical_rule(code).is_none())
			.collect::<Vec<_>>();
		assert!(
			missing.is_empty(),
			"dictionary null_flavors rules missing from catalog: {missing:?}"
		);

		let mismatched = dictionary_rules
			.iter()
			.filter(|(code, expected)| {
				null_flavors_source_hash_for_rule(code) != Some(*expected)
			})
			.collect::<Vec<_>>();
		assert!(
			mismatched.is_empty(),
			"dictionary null_flavors source hashes differ from catalog: {mismatched:?}"
		);
	}

	#[test]
	fn dictionary_rule_metadata_audit_has_no_catalog_gaps() {
		let required_codes = ich_required_dictionary_codes()
			.into_iter()
			.chain(fda_required_dictionary_codes())
			.chain(mfds_required_dictionary_codes())
			.map(|(code, _)| code)
			.collect::<Vec<_>>();
		let date_time_codes = CLASSIFIED_ICH_DATE_TIME_FUTURE_RULES
			.iter()
			.map(|code| (*code).to_string())
			.collect::<Vec<_>>();
		let max_length_codes = all_max_length_dictionary_rules()
			.into_iter()
			.map(|(code, _)| code)
			.collect::<Vec<_>>();
		let allowed_values_codes = all_allowed_values_dictionary_rules()
			.into_iter()
			.map(|(code, _)| code)
			.collect::<Vec<_>>();
		let vocabulary_codes = all_vocabulary_dictionary_rules()
			.into_iter()
			.map(|(code, _)| code)
			.collect::<Vec<_>>();
		let null_flavor_codes = all_null_flavors_dictionary_rules()
			.into_iter()
			.map(|(code, _)| code)
			.collect::<Vec<_>>();

		let buckets = [
			("required", required_codes),
			("date_time", date_time_codes),
			("max_length", max_length_codes),
			("allowed_values", allowed_values_codes),
			("vocabulary", vocabulary_codes),
			("null_flavors", null_flavor_codes),
		];
		let gaps = buckets
			.into_iter()
			.filter_map(|(bucket, codes)| {
				let missing = codes
					.into_iter()
					.filter(|code| find_canonical_rule(code).is_none())
					.collect::<Vec<_>>();
				(!missing.is_empty()).then_some((bucket.to_string(), missing))
			})
			.collect::<BTreeMap<_, _>>();

		assert_eq!(
			gaps,
			BTreeMap::<String, Vec<String>>::new(),
			"dictionary rule metadata must be catalog-backed with no unclassified gaps"
		);
	}

	#[test]
	fn ich_date_time_future_date_catalog_coverage_is_stable() {
		let actual_future_rules = VALIDATION_RULES
			.iter()
			.map(|rule| rule.code)
			.filter(|code| {
				code.starts_with("ICH.") && code.contains(".FUTURE_DATE.")
			})
			.map(str::to_string)
			.collect::<BTreeSet<_>>();
		let expected_future_rules = CLASSIFIED_ICH_DATE_TIME_FUTURE_RULES
			.iter()
			.map(|code| (*code).to_string())
			.collect::<BTreeSet<_>>();

		assert_eq!(
			actual_future_rules, expected_future_rules,
			"ICH FUTURE_DATE catalog rules changed; update Date/Time dictionary coverage classification"
		);
	}

	#[test]
	fn ich_dictionary_required_candidates_are_catalog_backed() {
		let missing = ich_required_dictionary_codes()
			.into_iter()
			.map(|(code, _)| code)
			.filter(|code| find_canonical_rule(code).is_none())
			.collect::<Vec<_>>();

		assert_eq!(
			missing,
			Vec::<String>::new(),
			"ICH dictionary required catalog gap changed"
		);
	}

	#[test]
	fn ich_conditional_dictionary_rules_are_catalog_conditioned() {
		let always = ich_conditional_dictionary_codes()
			.into_iter()
			.map(|(code, _)| code)
			.filter(|code| {
				find_canonical_rule(code)
					.map(|rule| rule.condition == RuleCondition::Always)
					.unwrap_or(true)
			})
			.collect::<Vec<_>>();

		assert_eq!(
			always,
			Vec::<String>::new(),
			"ICH conditional mandatory catalog rules must not be unconditional"
		);
	}

	#[test]
	fn ich_conditional_dictionary_condition_text_matches_catalog() {
		let mismatches = ich_conditional_dictionary_codes()
			.into_iter()
			.filter_map(|(code, dictionary_text)| {
				let catalog_text = condition_text_for_code(&code);
				(catalog_text != Some(dictionary_text.as_str())).then_some((
					code,
					dictionary_text,
					catalog_text.map(str::to_string),
				))
			})
			.collect::<Vec<_>>();

		assert_eq!(
			mismatches,
			Vec::<(String, String, Option<String>)>::new(),
			"ICH conditional mandatory condition_text catalog drift changed"
		);
	}

	#[test]
	fn ich_non_conditional_required_dictionary_rules_are_unconditional() {
		let conditioned = ich_non_conditional_required_dictionary_codes()
			.into_iter()
			.filter_map(|code| {
				let rule = find_canonical_rule(&code)?;
				(rule.condition != RuleCondition::Always)
					.then_some((code, rule.condition.as_str().to_string()))
			})
			.collect::<Vec<_>>();

		assert_eq!(
			conditioned,
			Vec::<(String, String)>::new(),
			"ICH mandatory/required dictionary rules must not carry catalog conditions"
		);
	}

	#[test]
	fn ich_catalog_required_extras_are_explicitly_classified() {
		let dictionary_required = ich_required_dictionary_codes()
			.into_iter()
			.map(|(code, _)| code)
			.collect::<BTreeSet<_>>();
		let catalog_required = VALIDATION_RULES
			.iter()
			.map(|rule| rule.code)
			.filter(|code| code.starts_with("ICH.") && code.ends_with(".REQUIRED"))
			.map(str::to_string)
			.collect::<BTreeSet<_>>();
		let actual_extras = catalog_required
			.difference(&dictionary_required)
			.cloned()
			.collect::<BTreeSet<_>>();
		let expected_extras = CLASSIFIED_ICH_CATALOG_REQUIRED_EXTRAS
			.iter()
			.map(|code| (*code).to_string())
			.collect::<BTreeSet<_>>();

		assert_eq!(
			actual_extras, expected_extras,
			"ICH catalog REQUIRED extras must be reviewed and explicitly classified"
		);
	}

	#[test]
	fn dictionary_required_rules_are_catalog_backed() {
		let cases = [
			("ICH", ich_required_dictionary_codes(), Vec::<String>::new()),
			("FDA", fda_required_dictionary_codes(), Vec::<String>::new()),
			("MFDS", mfds_required_dictionary_codes(), Vec::new()),
		];

		for (authority, codes, expected_missing) in cases {
			let missing = codes
				.into_iter()
				.map(|(code, _)| code)
				.filter(|code| find_canonical_rule(code).is_none())
				.collect::<Vec<_>>();

			assert_eq!(
				missing, expected_missing,
				"{authority} dictionary required catalog gap changed"
			);
		}
	}

	fn policy_allows_null_flavor(policy: ValuePolicy) -> bool {
		matches!(
			policy,
			ValuePolicy::NonEmptyOrNullFlavor
				| ValuePolicy::FdaRaceCodeOrNullFlavor
				| ValuePolicy::FdaEthnicityCodeOrNullFlavor
				| ValuePolicy::FdaGk10aCodeOrNa
				| ValuePolicy::FdaBooleanStringOrNullFlavor
		)
	}

	#[test]
	fn dictionary_null_flavor_required_rules_use_null_flavor_policy() {
		let expected_invalid = Vec::new();
		let invalid = ich_required_dictionary_codes()
			.into_iter()
			.chain(fda_required_dictionary_codes())
			.chain(mfds_required_dictionary_codes())
			.into_iter()
			.filter(|(_, null_flavors)| !null_flavors.is_empty())
			.filter(|(code, _)| find_canonical_rule(code).is_some())
			.filter_map(|(code, _)| {
				let policy = value_policy_for_code(&code)?;
				(!policy_allows_null_flavor(policy)).then_some((code, policy))
			})
			.collect::<Vec<_>>();

		assert_eq!(
			invalid, expected_invalid,
			"dictionary nullFlavor required policy gap changed"
		);
	}

	#[test]
	fn dictionary_null_flavor_required_rules_have_value_policy_bindings() {
		let expected_missing = Vec::<String>::new();
		let missing = ich_required_dictionary_codes()
			.into_iter()
			.chain(fda_required_dictionary_codes())
			.chain(mfds_required_dictionary_codes())
			.into_iter()
			.filter(|(_, null_flavors)| !null_flavors.is_empty())
			.filter(|(code, _)| find_canonical_rule(code).is_some())
			.map(|(code, _)| code)
			.filter(|code| value_policy_for_code(code).is_none())
			.collect::<Vec<_>>();

		assert_eq!(
			missing, expected_missing,
			"dictionary nullFlavor required policy bindings missing"
		);
	}

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
			let key = (rule.code, rule.authority.as_str(), rule.section);
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
	fn export_only_rules_are_not_validation_rules() {
		let export_only_codes = [
			"ICH.XML.STRUCTURAL.EMPTY.PRUNE",
			"ICH.XML.OPTIONAL.PATH.EMPTY.PRUNE",
			"ICH.XML.PLACEHOLDER.VALUE.PRUNE",
			"ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE",
			"ICH.XML.RACE.NI.PRUNE",
			"ICH.XML.RACE.EMPTY.PRUNE",
			"ICH.XML.GK11.EMPTY.PRUNE",
			"ICH.XML.XSI_TYPE.NORMALIZE",
		];
		let validation_codes: HashSet<_> = canonical_rules_all()
			.into_iter()
			.map(|rule| rule.code)
			.collect();
		for code in export_only_codes {
			assert!(
				!validation_codes.contains(code),
				"export-only policy code should not be a validation rule: {code}"
			);
		}
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
		let fda_rules = canonical_rules_for_authority(RegulatoryAuthority::Fda);
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
			"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
			RuleFacts {
				mfds_drug_foreign_non_kr: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(is_rule_condition_satisfied(
			"MFDS.C.3.1.KR.1.REQUIRED",
			RuleFacts {
				mfds_sender_type_is_health_professional: Some(true),
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
			Some("false"),
			None,
			RuleFacts::default()
		));
		assert!(!is_rule_value_valid(
			"FDA.C.1.12.REQUIRED",
			Some("1"),
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
		assert!(is_rule_value_valid(
			"FDA.C.1.7.1.REQUIRED",
			Some("5"),
			None,
			RuleFacts {
				fda_combination_product_true: Some(false),
				fda_fulfil_expedited_criteria: Some(true),
				..RuleFacts::default()
			}
		));
		assert!(!is_rule_value_valid(
			"FDA.C.1.7.1.REQUIRED",
			Some("3"),
			None,
			RuleFacts::default()
		));
	}

	#[test]
	fn conditioned_issue_codes_own_their_value_policy() {
		for (value, null_flavor) in [
			(Some("true"), None),
			(Some("false"), None),
			(Some("1"), None),
			(None, Some("NI")),
			(None, None),
		] {
			assert_eq!(
				is_rule_value_valid(
					"FDA.C.1.12.RECOMMENDED",
					value,
					null_flavor,
					RuleFacts::default(),
				),
				is_rule_value_valid(
					"FDA.C.1.12.REQUIRED",
					value,
					null_flavor,
					RuleFacts::default(),
				),
				"conditioned issue code must own the same policy for value={value:?}, null_flavor={null_flavor:?}",
			);
		}
	}

	#[test]
	fn ich_companion_conditions_are_catalog_executable() {
		let cases = [
			(
				"ICH.C.1.11.2.REQUIRED",
				RuleFacts {
					ich_nullification_code_present: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.C.3.2.REQUIRED",
				RuleFacts {
					ich_sender_organization_required: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.D.2.2a.REQUIRED",
				RuleFacts {
					ich_age_unit_present: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.D.2.2b.REQUIRED",
				RuleFacts {
					ich_age_value_present: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.D.2.2.1a.REQUIRED",
				RuleFacts {
					ich_gestation_unit_present: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.D.2.2.1b.REQUIRED",
				RuleFacts {
					ich_gestation_value_present: Some(true),
					..RuleFacts::default()
				},
			),
			(
				"ICH.D.9.3.REQUIRED",
				RuleFacts {
					ich_date_of_death_present: Some(true),
					..RuleFacts::default()
				},
			),
		];

		for (code, true_facts) in cases {
			assert!(!is_rule_condition_satisfied(code, RuleFacts::default()));
			assert!(is_rule_condition_satisfied(code, true_facts));
		}
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
}
