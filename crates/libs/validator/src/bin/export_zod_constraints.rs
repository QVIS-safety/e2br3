use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use validator::{
	portable_constraints, portable_field_bindings, PortableConstraint,
	PortableFieldBinding,
};

fn render_typescript(rules: &[PortableConstraint]) -> String {
	let json = serde_json::to_string_pretty(rules)
		.expect("portable constraints should serialize");
	format!(
		"// Generated from the backend validation Catalog. Do not edit.\n\
export const catalogConstraints = {json} as const;\n"
	)
}

fn render_bindings_typescript(bindings: &[&PortableFieldBinding]) -> String {
	let mut bindings = bindings.to_vec();
	bindings.sort_by_key(|binding| {
		(binding.section, binding.frontend_path, binding.rule_codes)
	});
	let json = serde_json::to_string_pretty(&bindings)
		.expect("portable bindings should serialize");
	format!(
		"// Generated from the backend portable field manifest. Do not edit.\n\
export type GeneratedCatalogBinding = {{\n\
  section: string;\n\
  frontendPath: string;\n\
  requestPath: string;\n\
  valueType: \"string\" | \"boolean\" | \"number\";\n\
  ruleCodes: readonly string[];\n\
  nullFlavorPath: string | null;\n\
}};\n\
export const catalogBindings: readonly GeneratedCatalogBinding[] = {json};\n"
	)
}

fn write_output(path: &Path, contents: &str) -> Result<(), String> {
	if fs::read_to_string(path).ok().as_deref() == Some(contents) {
		return Ok(());
	}
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).map_err(|err| {
			format!("failed to create {}: {err}", parent.display())
		})?;
	}
	fs::write(path, contents)
		.map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn check_output(path: &Path, contents: &str) -> Result<(), String> {
	match fs::read_to_string(path) {
		Ok(existing) if existing == contents => Ok(()),
		Ok(_) | Err(_) => Err(format!(
			"generated catalog artifact is stale: {}",
			path.display()
		)),
	}
}

enum Mode {
	Write,
	Check,
}

struct Args {
	mode: Mode,
	constraints_path: PathBuf,
	bindings_path: PathBuf,
}

fn parse_args() -> Result<Args, String> {
	let args = env::args().skip(1).collect::<Vec<_>>();
	match args.as_slice() {
		[constraints_flag, constraints_path, bindings_flag, bindings_path]
			if constraints_flag == "--constraints-output"
				&& bindings_flag == "--bindings-output" =>
		{
			Ok(Args {
				mode: Mode::Write,
				constraints_path: constraints_path.into(),
				bindings_path: bindings_path.into(),
			})
		}
		[constraints_flag, constraints_path, bindings_flag, bindings_path]
			if constraints_flag == "--check-constraints"
				&& bindings_flag == "--check-bindings" =>
		{
			Ok(Args {
				mode: Mode::Check,
				constraints_path: constraints_path.into(),
				bindings_path: bindings_path.into(),
			})
		}
		_ => Err(
			"expected --constraints-output <path> --bindings-output <path> or \
--check-constraints <path> --check-bindings <path>"
				.to_string(),
		),
	}
}

fn run() -> Result<(), String> {
	let args = parse_args()?;
	let constraints = render_typescript(&portable_constraints());
	let bindings = render_bindings_typescript(&portable_field_bindings());
	match args.mode {
		Mode::Write => {
			write_output(&args.constraints_path, &constraints)?;
			write_output(&args.bindings_path, &bindings)
		}
		Mode::Check => {
			check_output(&args.constraints_path, &constraints)?;
			check_output(&args.bindings_path, &bindings)
		}
	}
}

fn main() {
	if let Err(message) = run() {
		eprintln!("{message}");
		std::process::exit(1);
	}
}

#[cfg(test)]
mod tests {
	use super::{render_bindings_typescript, render_typescript};
	use validator::{portable_constraints, portable_field_bindings};

	#[test]
	fn renders_all_section_bindings_deterministically() {
		let first = render_bindings_typescript(&portable_field_bindings());
		let second = render_bindings_typescript(&portable_field_bindings());
		assert_eq!(first, second);

		for section in [
			"CI", "RP", "SD", "LR", "SI", "DM", "DH", "AE", "LB", "DG", "NR",
		] {
			assert!(
				first.contains(&format!("\"section\": \"{section}\"")),
				"missing generated section {section}"
			);
		}
	}

	#[test]
	fn output_is_sorted_and_marks_file_generated() {
		let output = render_typescript(&portable_constraints());
		assert!(output.starts_with(
			"// Generated from the backend validation Catalog. Do not edit.\n"
		));
		assert!(output.contains("export const catalogConstraints ="));
		assert!(
			output.find("ICH.C.1.1.LENGTH.MAX")
				< output.find("ICH.C.1.2.ALLOWED.VALUE")
		);
		assert!(output.contains("FDA.C.1.7.1.LENGTH.MAX"));
		assert!(output.contains("MFDS.C.2.r.4.KR.1.LENGTH.MAX"));
	}
}
