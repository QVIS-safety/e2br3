use serde_json::Value;
use std::path::PathBuf;

fn snapshot_path() -> PathBuf {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("rules/source/fda/core_regional_rules.normalized.json");
	path
}

#[test]
fn fda_source_snapshot_is_present_and_profile_partitioned() {
	let path = snapshot_path();
	assert!(
		path.exists(),
		"missing FDA source snapshot: {}",
		path.display()
	);

	let raw = std::fs::read_to_string(&path).expect("read source snapshot");
	let doc: Value = serde_json::from_str(&raw).expect("parse source snapshot JSON");
	let record_count = doc["record_count"].as_u64().unwrap_or(0);
	assert!(record_count > 0, "snapshot has no normalized rows");

	let counts = doc["counts_by_profile"]
		.as_object()
		.expect("counts_by_profile must be an object");
	assert!(
		counts.get("fda").and_then(Value::as_u64).unwrap_or(0) > 0,
		"expected FDA-specific rows in source snapshot"
	);
	assert!(
		counts.get("ich").and_then(Value::as_u64).unwrap_or(0) > 0,
		"expected ICH/core rows in source snapshot"
	);
	assert!(
		counts.get("unknown").is_none(),
		"unexpected unknown profile rows in source snapshot"
	);

	let rules = doc["rules"].as_array().expect("rules must be an array");
	for rule in rules {
		let profile = rule["profile"].as_str().unwrap_or_default();
		let tag = rule["tag_key"].as_str().unwrap_or_default();
		if profile == "fda" {
			assert!(
				tag.starts_with("FDA."),
				"fda profile row must be FDA-tagged: {tag}"
			);
		}
		if profile == "ich" {
			assert!(
				tag.starts_with("ICH.") || tag.starts_with("ACK."),
				"ich profile row must be ICH/ACK-tagged: {tag}"
			);
		}
	}
}
