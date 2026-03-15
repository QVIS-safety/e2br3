// Shared Section G policy used by exporter + case validators.

pub fn has_drug_characterization(value: &str) -> bool {
	!value.trim().is_empty()
}

pub fn has_medicinal_product(value: &str) -> bool {
	!value.trim().is_empty()
}

pub fn normalize_drug_characterization(value: &str) -> Option<&'static str> {
	match value.trim() {
		"1" => Some("1"),
		"2" => Some("2"),
		"3" => Some("3"),
		"4" => Some("4"),
		_ => None,
	}
}

pub fn drug_characterization_display_name(code: &str) -> &'static str {
	match code {
		"1" => "Suspect",
		"2" => "Concomitant",
		"3" => "Interacting",
		"4" => "Drug Not Administered",
		_ => "Concomitant",
	}
}

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
