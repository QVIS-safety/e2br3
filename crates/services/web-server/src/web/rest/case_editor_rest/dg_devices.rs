use super::super::common::{Error, Map, ModelManager, Result, Uuid, Value};
use lib_core::ctx::Ctx;

pub(super) async fn persist_devices(
	ctx: &Ctx,
	mm: &ModelManager,
	drug_id: Uuid,
	row: &Map<String, Value>,
) -> Result<()> {
	if let Some(value) = row.get("fdaDevices") {
		let devices = serde_json::from_value(value.clone()).map_err(|_| {
			Error::BadRequest {
				message: "Invalid FDA device information".into(),
			}
		})?;
		crate::web::rest::drug_sub_rest::persist_fda_devices(
			ctx, mm, drug_id, devices,
		)
		.await?;
	}
	Ok(())
}
