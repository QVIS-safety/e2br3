use crate::{Result, XmlValidationError};
use libxml::xpath::Context;

mod e;
mod g;
mod h;
pub(super) mod n;

pub(super) fn run(
	xpath: &mut Context,
	authority: lib_core::regulatory::RegulatoryAuthority,
	errors: &mut Vec<XmlValidationError>,
) -> Result<()> {
	n::run(xpath, authority, errors)?;
	e::run(xpath, errors)?;
	g::run(xpath, errors)?;
	h::run(xpath, errors)?;
	Ok(())
}
