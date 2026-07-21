use lib_core::authorization::{export_contract, policy_registry, Availability};

#[test]
fn catalog_hash_is_stable_for_canonical_registry_json() {
	let first = export_contract(policy_registry()).unwrap();
	let second = export_contract(policy_registry()).unwrap();
	assert_eq!(first.catalog_hash, second.catalog_hash);
	assert_eq!(first.canonical_json, second.canonical_json);
	assert_eq!(first.catalog_hash.len(), 64);
	assert!(first
		.catalog_hash
		.chars()
		.all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
	assert_eq!(
		first.catalog_hash,
		include_str!("snapshots/authorization_catalog.sha256").trim()
	);
}

#[test]
fn generated_pdf_rows_preserve_reviewed_order_and_availability() {
	let contract = export_contract(policy_registry()).unwrap();
	assert_eq!(
		contract.pdf_rows.first().unwrap().grant_id,
		"home.notice.read"
	);
	assert_eq!(
		contract.pdf_rows.last().unwrap().grant_id,
		"email.report_due.send"
	);
	assert_eq!(
		contract
			.pdf_rows
			.iter()
			.find(|row| row.grant_id == "email.report_due.read")
			.unwrap()
			.availability,
		Availability::Reserved
	);
	assert!(contract
		.pdf_rows
		.iter()
		.all(|row| row.grant_id != "settings.read"));
}

#[test]
fn generated_typescript_contains_registry_owned_symbols() {
	let contract = export_contract(policy_registry()).unwrap();
	assert!(contract
		.typescript
		.contains("CaseReviewToggle: \"case.review.toggle\""));
	assert!(contract
		.typescript
		.contains("CaseLockToggle: \"case.lock.toggle\""));
	assert!(contract
		.typescript
		.contains(&format!("  \"{}\";", contract.catalog_hash)));
}
