use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use validator::{portable_ich_constraints, PortableConstraint};

fn render_typescript(rules: &[PortableConstraint]) -> String {
	let json = serde_json::to_string_pretty(rules)
		.expect("portable constraints should serialize");
	format!(
		"// Generated from the backend validation Catalog. Do not edit.\n\
export const catalogConstraints = {json} as const;\n"
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
		Ok(_) | Err(_) => {
			Err(format!("catalog constraints are stale: {}", path.display()))
		}
	}
}

fn parse_args() -> Result<(String, PathBuf), String> {
	let mut args = env::args().skip(1);
	let mode = args
		.next()
		.ok_or_else(|| "expected --output <path> or --check <path>".to_string())?;
	let path = args
		.next()
		.map(PathBuf::from)
		.ok_or_else(|| format!("{mode} requires a path"))?;
	if args.next().is_some() || !matches!(mode.as_str(), "--output" | "--check") {
		return Err("expected --output <path> or --check <path>".to_string());
	}
	Ok((mode, path))
}

fn run() -> Result<(), String> {
	let (mode, path) = parse_args()?;
	let contents = render_typescript(&portable_ich_constraints());
	match mode.as_str() {
		"--output" => write_output(&path, &contents),
		"--check" => check_output(&path, &contents),
		_ => unreachable!("parse_args validates the mode"),
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
	use super::render_typescript;
	use validator::portable_ich_constraints;

	#[test]
	fn output_is_sorted_and_marks_file_generated() {
		let output = render_typescript(&portable_ich_constraints());
		assert!(output.starts_with(
			"// Generated from the backend validation Catalog. Do not edit.\n"
		));
		assert!(output.contains("export const catalogConstraints ="));
		assert!(
			output.find("ICH.C.1.1.LENGTH.MAX")
				< output.find("ICH.C.1.2.ALLOWED.VALUE")
		);
	}
}
