use lib_utils::envs::get_env;
use std::sync::OnceLock;

pub fn core_config() -> lib_utils::envs::Result<&'static CoreConfig> {
	static INSTANCE: OnceLock<lib_utils::envs::Result<CoreConfig>> = OnceLock::new();

	INSTANCE
		.get_or_init(CoreConfig::load_from_env)
		.as_ref()
		.map_err(Clone::clone)
}

#[allow(non_snake_case)]
pub struct CoreConfig {
	// -- Db
	pub DB_URL: String,

	// -- Web
	pub WEB_FOLDER: String,
}

impl CoreConfig {
	fn load_from_env() -> lib_utils::envs::Result<CoreConfig> {
		Ok(CoreConfig {
			// -- Db
			DB_URL: get_env("SERVICE_DB_URL")?,

			// -- Web
			WEB_FOLDER: get_env("SERVICE_WEB_FOLDER")?,
		})
	}
}
