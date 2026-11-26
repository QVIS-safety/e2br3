#[test]
fn every_bootstrap_null_flavor_column_has_an_allowed_value_check() {
	let sql = [
		include_str!("../../../../db/bootstrap/01-safetydb-schema.sql"),
		include_str!("../../../../db/bootstrap/03-safety-report-identification.sql"),
		include_str!("../../../../db/bootstrap/04-patient-information.sql"),
		include_str!("../../../../db/bootstrap/05-reactions.sql"),
		include_str!("../../../../db/bootstrap/06-test-results.sql"),
		include_str!("../../../../db/bootstrap/07-drug-information.sql"),
	]
	.concat();
	let columns: Vec<_> = sql
		.lines()
		.filter(|line| {
			line.contains("_null_flavor VARCHAR") && !line.contains("ADD COLUMN")
		})
		.collect();
	assert_eq!(columns.len(), 90, "NullFlavor storage inventory changed");
	for column in columns {
		assert!(
			column.contains("CHECK") && column.contains(" IN ("),
			"missing allowed-value CHECK: {column}"
		);
	}
}

#[test]
fn null_flavor_alignment_migration_covers_all_confirmed_mismatches() {
	let migration = include_str!(
		"../../../../db/migrations/20260803_align_null_flavor_allowed_values.sql"
	);
	assert_eq!(
		migration.matches("ADD CONSTRAINT ck_nf_").count(),
		40,
		"confirmed NullFlavor mismatch inventory changed"
	);
}
