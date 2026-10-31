use sha2::{Digest, Sha256};
use std::{env, error::Error, fs, path::Path};

fn hash_path(path: &Path, hash: &mut Sha256) -> Result<(), Box<dyn Error>> {
	println!("cargo:rerun-if-changed={}", path.display());
	if path.is_dir() {
		let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
		entries.sort_by_key(|entry| entry.file_name());
		for entry in entries {
			if entry.file_name() != "tests" && entry.file_name() != "rule_tests" {
				hash_path(&entry.path(), hash)?;
			}
		}
	} else {
		let name = path.to_str().ok_or("non-UTF8 validation source path")?;
		let bytes = fs::read(path)?;
		hash.update((name.len() as u64).to_le_bytes());
		hash.update(name.as_bytes());
		hash.update((bytes.len() as u64).to_le_bytes());
		hash.update(bytes);
	}
	Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
	env::set_current_dir(env::var("CARGO_MANIFEST_DIR")?)?;
	let mut hash = Sha256::new();
	// Include context loaders and embedded vocabularies as well as the rules themselves.
	for source in [
		"build.rs",
		"Cargo.toml",
		"src",
		"../lib-core/src",
		"../lib-core/Cargo.toml",
		"../../../Cargo.toml",
		"../../../Cargo.lock",
		"../../../registry/vocabularies",
	] {
		hash_path(Path::new(source), &mut hash)?;
	}
	println!("cargo:rustc-env=VALIDATOR_REVISION={:x}", hash.finalize());
	Ok(())
}
