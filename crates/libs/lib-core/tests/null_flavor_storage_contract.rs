#[test]
fn every_bootstrap_null_flavor_column_has_an_allowed_value_check() {
	let sql = include_str!("../../../../db/bootstrap/01-safetydb-schema.sql");
	let mut count = 0;
	for table in sql.split("CREATE TABLE public.").skip(1) {
		let definition = table.split("\n);").next().expect("table definition");
		for column in definition.lines().filter(|line| {
			line.contains("_null_flavor character varying")
				&& !line.contains("CONSTRAINT")
		}) {
			count += 1;
			let name = column.split_whitespace().next().expect("column name");
			assert!(
				definition.lines().any(|line| {
					line.contains("CHECK")
						&& line.contains(name)
						&& (line.contains("ANY (ARRAY[")
							|| line.contains(&format!("({name})::text = '")))
				}),
				"missing allowed-value CHECK: {column}"
			);
		}
	}
	assert_eq!(count, 90, "NullFlavor storage inventory changed");
}

#[test]
fn null_flavor_alignment_migration_covers_all_confirmed_mismatches() {
	let migration = include_str!(
		"../../../../tests/fixtures/prebaseline/20260803_align_null_flavor_allowed_values.sql"
	);
	assert_eq!(
		migration.matches("ADD CONSTRAINT ck_nf_").count(),
		40,
		"confirmed NullFlavor mismatch inventory changed"
	);
}
