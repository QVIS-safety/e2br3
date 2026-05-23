use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RegulatoryAuthority {
	Ich,
	Fda,
	Mfds,
}

impl RegulatoryAuthority {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Ich => "ich",
			Self::Fda => "fda",
			Self::Mfds => "mfds",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value.trim().to_ascii_lowercase().as_str() {
			"ich" => Some(Self::Ich),
			"fda" => Some(Self::Fda),
			"mfds" => Some(Self::Mfds),
			_ => None,
		}
	}

	pub fn from_case_profile(value: Option<&str>) -> Option<Self> {
		value.and_then(Self::parse)
	}

	pub fn default_message_receiver_identifier(self) -> &'static str {
		match self {
			Self::Fda => "CDER",
			Self::Ich => "ICHTEST",
			Self::Mfds => MFDS_MSG_RECEIVER_DOMESTIC,
		}
	}

	pub fn requires_fda_context(self) -> bool {
		matches!(self, Self::Fda)
	}

	pub fn requires_mfds_context(self) -> bool {
		matches!(self, Self::Mfds)
	}
}

pub const FDA_BATCH_RECEIVER_POSTMARKET: &str = "ZZFDA";
pub const FDA_BATCH_RECEIVER_PREMARKET: &str = "ZZFDA_PREMKT";
pub const FDA_MSG_RECEIVER_CDER: &str = "CDER";
pub const FDA_MSG_RECEIVER_CBER: &str = "CBER";
pub const FDA_MSG_RECEIVER_CDER_IND: &str = "CDER_IND";
pub const FDA_MSG_RECEIVER_CBER_IND: &str = "CBER_IND";
pub const FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE: &str = "CDER_IND_EXEMPT_BA_BE";
/// Marker string present in all MFDS N.1.4 batch receiver identifiers.
pub const MFDS_RECEIVER_IDENTIFIER: &str = "MFDS";
/// MFDS N.1.4 batch receiver identifiers (used for profile inference).
pub const MFDS_BATCH_RECEIVER_POSTMARKET_DOMESTIC: &str = "MFDS";
pub const MFDS_BATCH_RECEIVER_POSTMARKET_FOREIGN: &str = "MFDS_FR";
pub const MFDS_BATCH_RECEIVER_CLINICAL_TRIAL: &str = "MFDS_CT";
pub const MFDS_BATCH_RECEIVER_COMPASSIONATE_USE: &str = "MFDS_CU";

/// MFDS N.1.5 message receiver sub-type codes (used for validation branching).
pub const MFDS_MSG_RECEIVER_DOMESTIC: &str = "KR";
pub const MFDS_MSG_RECEIVER_FOREIGN: &str = "FR";
pub const MFDS_MSG_RECEIVER_CLINICAL_TRIAL: &str = "CT";
pub const MFDS_MSG_RECEIVER_COMPASSIONATE_USE: &str = "CU";

/// Known valid MFDS N.1.4 batch receiver codes.
pub const MFDS_KNOWN_BATCH_RECEIVERS: &[&str] = &[
	MFDS_BATCH_RECEIVER_POSTMARKET_DOMESTIC,
	MFDS_BATCH_RECEIVER_POSTMARKET_FOREIGN,
	MFDS_BATCH_RECEIVER_CLINICAL_TRIAL,
	MFDS_BATCH_RECEIVER_COMPASSIONATE_USE,
];

/// Returns true if the N.1.5 message receiver identifies MFDS domestic (KR) reporting.
pub fn is_mfds_domestic_receiver(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.eq_ignore_ascii_case(MFDS_MSG_RECEIVER_DOMESTIC))
		.unwrap_or(false)
}

/// Returns true if the N.1.5 message receiver identifies MFDS foreign postmarket (FR) reporting.
pub fn is_mfds_foreign_postmarket_receiver(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.eq_ignore_ascii_case(MFDS_MSG_RECEIVER_FOREIGN))
		.unwrap_or(false)
}

/// Returns true if the N.1.5 message receiver identifies MFDS clinical trial (CT) reporting.
pub fn is_mfds_clinical_trial_receiver(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.eq_ignore_ascii_case(MFDS_MSG_RECEIVER_CLINICAL_TRIAL))
		.unwrap_or(false)
}

/// Returns true if the N.1.5 message receiver identifies MFDS compassionate use (CU) reporting.
pub fn is_mfds_compassionate_use_receiver(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.eq_ignore_ascii_case(MFDS_MSG_RECEIVER_COMPASSIONATE_USE))
		.unwrap_or(false)
}

pub fn is_fda_batch_receiver(value: Option<&str>) -> bool {
	matches!(
		value,
		Some(FDA_BATCH_RECEIVER_POSTMARKET | FDA_BATCH_RECEIVER_PREMARKET)
	)
}

pub fn is_fda_postmarket_batch_receiver(value: Option<&str>) -> bool {
	value == Some(FDA_BATCH_RECEIVER_POSTMARKET)
}

pub fn is_fda_premarket_batch_receiver(value: Option<&str>) -> bool {
	value == Some(FDA_BATCH_RECEIVER_PREMARKET)
}

pub fn is_fda_message_receiver(value: Option<&str>) -> bool {
	matches!(
		value,
		Some(
			FDA_MSG_RECEIVER_CDER
				| FDA_MSG_RECEIVER_CBER
				| FDA_MSG_RECEIVER_CDER_IND
				| FDA_MSG_RECEIVER_CBER_IND
				| FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE
		)
	)
}

pub fn is_fda_postmarket_message_receiver(value: Option<&str>) -> bool {
	matches!(value, Some(FDA_MSG_RECEIVER_CDER | FDA_MSG_RECEIVER_CBER))
}

pub fn is_fda_ind_message_receiver(value: Option<&str>) -> bool {
	matches!(
		value,
		Some(FDA_MSG_RECEIVER_CDER_IND | FDA_MSG_RECEIVER_CBER_IND)
	)
}

pub fn is_fda_premarket_message_receiver(value: Option<&str>) -> bool {
	matches!(
		value,
		Some(
			FDA_MSG_RECEIVER_CDER_IND
				| FDA_MSG_RECEIVER_CBER_IND
				| FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE
		)
	)
}

pub fn is_fda_pre_anda_message_receiver(value: Option<&str>) -> bool {
	value == Some(FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE)
}

pub fn is_mfds_receiver(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.to_ascii_uppercase().contains(MFDS_RECEIVER_IDENTIFIER))
		.unwrap_or(false)
}

pub fn infer_regulatory_authority_from_receivers(
	batch_receiver: Option<&str>,
	message_receiver: Option<&str>,
) -> RegulatoryAuthority {
	if is_mfds_receiver(batch_receiver) || is_mfds_receiver(message_receiver) {
		return RegulatoryAuthority::Mfds;
	}
	if is_fda_batch_receiver(batch_receiver)
		|| is_fda_message_receiver(message_receiver)
	{
		return RegulatoryAuthority::Fda;
	}
	RegulatoryAuthority::Ich
}
