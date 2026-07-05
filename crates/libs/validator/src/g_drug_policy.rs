pub use lib_core::xml::export::policy::{
	drug_characterization_display_name, has_drug_characterization,
	has_medicinal_product, normalize_drug_characterization,
};

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_drug_characterization_rejects_missing_or_invalid() {
		assert_eq!(normalize_drug_characterization(""), None);
		assert_eq!(normalize_drug_characterization("99"), None);
	}

	#[test]
	fn normalize_drug_characterization_preserves_valid() {
		assert_eq!(normalize_drug_characterization("1"), Some("1"));
		assert_eq!(normalize_drug_characterization("2"), Some("2"));
		assert_eq!(normalize_drug_characterization("3"), Some("3"));
		assert_eq!(normalize_drug_characterization("4"), Some("4"));
	}
}
