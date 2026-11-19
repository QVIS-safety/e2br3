mod config;
pub mod pwd;
pub mod token;

use config::auth_config;

pub fn init() -> lib_utils::envs::Result<()> {
	auth_config().map(|_| ())
}
