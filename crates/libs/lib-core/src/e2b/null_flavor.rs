use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NullFlavor {
	NI,
	UNK,
	ASKU,
	NASK,
	MSK,
}

impl NullFlavor {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::NI => "NI",
			Self::UNK => "UNK",
			Self::ASKU => "ASKU",
			Self::NASK => "NASK",
			Self::MSK => "MSK",
		}
	}

	pub fn is_one_of(self, allowed: &[NullFlavor]) -> bool {
		allowed.contains(&self)
	}
}

impl fmt::Display for NullFlavor {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNullFlavorError {
	value: String,
}

impl fmt::Display for ParseNullFlavorError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "invalid nullFlavor `{}`", self.value)
	}
}

impl std::error::Error for ParseNullFlavorError {}

impl FromStr for NullFlavor {
	type Err = ParseNullFlavorError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value.trim() {
			"NI" => Ok(Self::NI),
			"UNK" => Ok(Self::UNK),
			"ASKU" => Ok(Self::ASKU),
			"NASK" => Ok(Self::NASK),
			"MSK" => Ok(Self::MSK),
			trimmed => Err(ParseNullFlavorError {
				value: trimmed.to_string(),
			}),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum E2bNullFlavorValue<T> {
	Value { value: T },
	NullFlavor { null_flavor: NullFlavor },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum E2bNullFlavorPartsError {
	ValueAndNullFlavor,
	InvalidNullFlavor(ParseNullFlavorError),
}

impl fmt::Display for E2bNullFlavorPartsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ValueAndNullFlavor => {
				f.write_str("value and nullFlavor cannot both be present")
			}
			Self::InvalidNullFlavor(err) => err.fmt(f),
		}
	}
}

impl std::error::Error for E2bNullFlavorPartsError {}

impl From<ParseNullFlavorError> for E2bNullFlavorPartsError {
	fn from(value: ParseNullFlavorError) -> Self {
		Self::InvalidNullFlavor(value)
	}
}

impl<T> E2bNullFlavorValue<T> {
	pub fn from_parts(
		value: Option<T>,
		null_flavor: Option<&str>,
	) -> Result<Option<Self>, E2bNullFlavorPartsError> {
		let null_flavor =
			null_flavor.map(str::trim).filter(|value| !value.is_empty());
		match (value, null_flavor) {
			(Some(_), Some(_)) => Err(E2bNullFlavorPartsError::ValueAndNullFlavor),
			(Some(value), None) => Ok(Some(Self::Value { value })),
			(None, Some(null_flavor)) => Ok(Some(Self::NullFlavor {
				null_flavor: null_flavor.parse()?,
			})),
			(None, None) => Ok(None),
		}
	}

	pub fn into_parts(self) -> (Option<T>, Option<String>) {
		match self {
			Self::Value { value } => (Some(value), None),
			Self::NullFlavor { null_flavor } => {
				(None, Some(null_flavor.as_str().to_string()))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_known_null_flavors() {
		assert_eq!("MSK".parse::<NullFlavor>().unwrap(), NullFlavor::MSK);
		assert_eq!("ASKU".parse::<NullFlavor>().unwrap(), NullFlavor::ASKU);
	}

	#[test]
	fn rejects_unknown_null_flavor() {
		assert!("OTH".parse::<NullFlavor>().is_err());
	}

	#[test]
	fn rejects_value_and_null_flavor_together() {
		let err =
			E2bNullFlavorValue::from_parts(Some("value"), Some("UNK")).unwrap_err();
		assert_eq!(err, E2bNullFlavorPartsError::ValueAndNullFlavor);
	}

	#[test]
	fn splits_null_flavor_back_to_storage_parts() {
		let field = E2bNullFlavorValue::<String>::from_parts(None, Some("UNK"))
			.unwrap()
			.unwrap();
		assert_eq!(field.into_parts(), (None, Some("UNK".to_string())));
	}
}
