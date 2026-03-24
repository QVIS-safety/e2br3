use crate::validation::ValidationProfile;
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

	pub fn from_validation_profile(profile: ValidationProfile) -> Self {
		match profile {
			ValidationProfile::Ich => Self::Ich,
			ValidationProfile::Fda => Self::Fda,
			ValidationProfile::Mfds => Self::Mfds,
		}
	}

	pub fn to_validation_profile(self) -> ValidationProfile {
		match self {
			Self::Ich => ValidationProfile::Ich,
			Self::Fda => ValidationProfile::Fda,
			Self::Mfds => ValidationProfile::Mfds,
		}
	}

	pub fn default_message_receiver_identifier(self) -> &'static str {
		match self {
			Self::Fda => "CDER",
			Self::Ich => "ICHTEST",
			Self::Mfds => "MFDS",
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
pub const MFDS_RECEIVER_IDENTIFIER: &str = "MFDS";

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
	RegulatoryAuthority::Fda
}
