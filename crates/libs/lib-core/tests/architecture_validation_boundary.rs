use std::fs;
use std::path::Path;

#[test]
fn lib_core_runtime_boundaries_do_not_depend_on_validation_module() {
	let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
	let guarded_files = [
		"src/xml/import.rs",
		"src/xml/mod.rs",
		"src/xml/xml_validation/mod.rs",
		"src/xml/ich/validation.rs",
		"src/xml/fda/validation.rs",
		"src/xml/mfds/validation.rs",
		"src/xml/export/shared/patch_doc.rs",
		"src/xml/export/sections/e.rs",
		"src/xml/export/sections/g.rs",
		"src/model/case_validation_report_cache.rs",
		"src/model/case_validation_summary.rs",
	];

	let offenders = guarded_files
		.into_iter()
		.filter_map(|relative| {
			let contents = fs::read_to_string(crate_root.join(relative)).ok()?;
			contents
				.contains("crate::validation::")
				.then_some(relative.to_string())
				.or_else(|| {
					contents
						.contains("crate::validation;")
						.then_some(relative.to_string())
				})
				.or_else(|| {
					contents
						.contains("crate::validation::{")
						.then_some(relative.to_string())
				})
		})
		.collect::<Vec<_>>();

	assert!(
		offenders.is_empty(),
		"lib-core runtime boundary files must not depend on crate::validation: {offenders:?}"
	);
}
