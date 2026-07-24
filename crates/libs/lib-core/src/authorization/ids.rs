use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierError {
	value: String,
}

impl IdentifierError {
	pub fn value(&self) -> &str {
		&self.value
	}
}

impl Display for IdentifierError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			formatter,
			"non-canonical authorization identifier: {:?}",
			self.value
		)
	}
}

impl std::error::Error for IdentifierError {}

macro_rules! authorization_id {
	($name:ident) => {
		#[derive(
			Debug,
			Clone,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			Serialize,
			Deserialize,
		)]
		#[serde(transparent)]
		pub struct $name(String);

		impl $name {
			pub fn parse(value: impl Into<String>) -> Result<Self, IdentifierError> {
				let value = value.into();
				if is_canonical_identifier(&value) {
					Ok(Self(value))
				} else {
					Err(IdentifierError { value })
				}
			}

			pub fn as_str(&self) -> &str {
				&self.0
			}
		}

		impl Borrow<str> for $name {
			fn borrow(&self) -> &str {
				self.as_str()
			}
		}

		impl Display for $name {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
				formatter.write_str(self.as_str())
			}
		}
	};
}

authorization_id!(GrantId);
authorization_id!(ActionId);
authorization_id!(FactId);

fn is_canonical_identifier(value: &str) -> bool {
	let mut segment_count = 0;
	for segment in value.split('.') {
		segment_count += 1;
		let mut chars = segment.chars();
		if !chars.next().is_some_and(|ch| ch.is_ascii_lowercase()) {
			return false;
		}
		if !chars
			.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
		{
			return false;
		}
	}
	segment_count >= 2
}
