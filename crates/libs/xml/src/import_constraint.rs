use crate::{Error, Result};
use input_contracts::InputRecord;
use libxml::{tree::Document, xpath};
use std::ops::{Deref, DerefMut};

/// XML readers collect input; input-contracts validates the complete record.
pub(crate) struct Context {
	xpath: xpath::Context,
	pub(crate) inputs: InputRecord,
}

impl Context {
	pub(crate) fn new(doc: &Document) -> std::result::Result<Self, ()> {
		let xpath = xpath::Context::new(doc)?;
		xpath.register_namespace(
			"xsi",
			"http://www.w3.org/2001/XMLSchema-instance",
		)?;
		Ok(Self {
			xpath,
			inputs: InputRecord::default(),
		})
	}

	pub(crate) fn validate(&self) -> Result<()> {
		validate(&self.inputs)
	}
}

impl Deref for Context {
	type Target = xpath::Context;
	fn deref(&self) -> &Self::Target {
		&self.xpath
	}
}

impl DerefMut for Context {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.xpath
	}
}

pub(crate) fn validate(inputs: &InputRecord) -> Result<()> {
	inputs.validate().map_err(|error| Error::InvalidXml {
		message: format!(
			"{}: {}: {}",
			error.path, error.issue.code, error.issue.message
		),
		line: None,
		column: None,
	})
}

pub(crate) fn decimal(
	value: Option<String>,
	field: &str,
) -> Result<Option<rust_decimal::Decimal>> {
	value
		.map(|value| {
			rust_decimal::Decimal::from_str_exact(
				&input_contracts::normalize_decimal_lexeme(&value),
			)
			.map_err(|_| Error::InvalidXml {
				message: format!(
					"{field}: numeric value cannot be stored without loss"
				),
				line: None,
				column: None,
			})
		})
		.transpose()
}

pub(crate) fn date(
	value: Option<String>,
	field: &str,
) -> Result<Option<sqlx::types::time::Date>> {
	value
		.map(|value| {
			let parsed = if value.len() == 8
				&& value.bytes().all(|byte| byte.is_ascii_digit())
			{
				crate::import_sections::shared::parse_date(value)
			} else {
				None
			};
			parsed.ok_or_else(|| Error::InvalidXml {
				message: format!("{field}: date cannot be stored without loss"),
				line: None,
				column: None,
			})
		})
		.transpose()
}

#[cfg(test)]
mod tests {
	#[test]
	fn supplied_numbers_and_dates_never_silently_become_absent() {
		assert_eq!(super::decimal(None, "age").unwrap(), None);
		assert_eq!(
			super::decimal(Some("+.5".into()), "age")
				.unwrap()
				.unwrap()
				.to_string(),
			"0.5"
		);
		assert!(super::decimal(
			Some("79228162514264337593543950336".into()),
			"dose"
		)
		.is_err());
		assert!(super::date(Some("20230229".into()), "birth date").is_err());
		assert!(super::date(Some("2024".into()), "birth date").is_err());
		assert!(super::date(Some("20240229123000".into()), "birth date").is_err());
		assert!(super::decimal(
			Some("0.00000000000000000000000000001".into()),
			"dose"
		)
		.is_err());
	}
}
