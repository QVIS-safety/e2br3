use lib_core::xml::validate::{canonical_rules_for_profile, ValidationProfile};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fda_snapshot_path() -> PathBuf {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("rules/source/fda/core_regional_rules.normalized.json");
	path
}

fn baseline_path() -> PathBuf {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("rules/source/fda/unmapped_rejection_warning_tags.baseline.txt");
	path
}

fn normalize_tag_for_parity(tag: &str) -> String {
	let mut t = tag.trim().to_ascii_uppercase();
	t = t.replace("FDA.G2.", "FDA.G.2.");
	// Repeating markers (e.g. FDA.D.11.r, FDA.C.5.6.r) are mapped to base field.
	if let Some(idx) = t.rfind(".R") {
		let suffix = &t[idx..];
		let repeat_only = suffix == ".R"
			|| suffix
				.strip_prefix(".R.")
				.map(|s| {
					let mut chars = s.chars();
					let digits: String =
						chars.by_ref().take_while(|c| c.is_ascii_digit()).collect();
					if digits.is_empty() {
						return false;
					}
					let rest: String = chars.collect();
					rest.is_empty()
						|| (rest.len() == 1
							&& rest.chars().all(|c| c.is_ascii_uppercase()))
				})
				.unwrap_or(false);
		if repeat_only {
			t.truncate(idx);
		}
	}
	t
}

fn load_snapshot_fda_rejection_warning_tags() -> BTreeSet<String> {
	let raw = std::fs::read_to_string(fda_snapshot_path())
		.expect("read FDA source snapshot");
	let doc: Value = serde_json::from_str(&raw).expect("parse FDA source snapshot");
	let rules = doc["rules"].as_array().expect("snapshot rules array");
	rules
		.iter()
		.filter(|r| r["profile"].as_str() == Some("fda"))
		.filter(|r| r["sheet"].as_str() == Some("Rejection and Warning Rules"))
		.filter(|r| {
			let severity = r["severity"]
				.as_str()
				.map(str::trim)
				.unwrap_or("")
				.to_ascii_lowercase();
			let message = r["message"].as_str().map(str::trim).unwrap_or("");
			matches!(severity.as_str(), "rejection" | "warning")
				|| !message.is_empty()
		})
		.filter_map(|r| r["tag_key"].as_str())
		.filter(|tag| tag.starts_with("FDA."))
		.map(normalize_tag_for_parity)
		.collect()
}

fn load_catalog_fda_codes() -> BTreeSet<String> {
	canonical_rules_for_profile(ValidationProfile::Fda)
		.into_iter()
		.map(|r| r.code.to_ascii_uppercase())
		.filter(|code| code.starts_with("FDA."))
		.collect()
}

fn load_unmapped_baseline() -> BTreeSet<String> {
	let raw =
		std::fs::read_to_string(baseline_path()).expect("read unmapped baseline");
	raw.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
		.map(ToOwned::to_owned)
		.collect()
}

#[test]
fn fda_rejection_warning_tag_parity_tracks_baseline() {
	let snapshot_tags = load_snapshot_fda_rejection_warning_tags();
	assert!(
		!snapshot_tags.is_empty(),
		"expected FDA rejection/warning tags from source snapshot"
	);

	let catalog_codes = load_catalog_fda_codes();
	assert!(
		!catalog_codes.is_empty(),
		"expected catalog FDA rules for parity check"
	);

	let unmapped: BTreeSet<String> = snapshot_tags
		.iter()
		.filter(|tag| !catalog_codes.iter().any(|code| code.starts_with(*tag)))
		.cloned()
		.collect();
	let baseline = load_unmapped_baseline();

	assert_eq!(
		unmapped, baseline,
		"FDA source/catalog parity drift: review unmapped_rejection_warning_tags.baseline.txt after intentional rule updates"
	);
}
