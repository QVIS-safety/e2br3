use crate::ctx::Ctx;
use crate::model::admin_settings::AdminSettingsBmc;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{ModelManager, Result};
use serde_json::Value;

const SETTINGS_KEY: &str = "system";
const DEFAULT_IDENTIFIER: &str = "ICSR";
const DEFAULT_PADDING: usize = 6;

pub struct GeneratedCaseNumber {
	pub safety_report_id: String,
	pub worldwide_unique_id: String,
}

fn setting_string(settings: Option<&Value>, key: &str) -> Option<String> {
	settings
		.and_then(|value| value.get(key))
		.and_then(Value::as_str)
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(ToOwned::to_owned)
}

fn setting_padding(settings: Option<&Value>) -> usize {
	settings
		.and_then(|value| value.get("case_number_padding"))
		.and_then(Value::as_i64)
		.filter(|value| *value > 0)
		.map(|value| value as usize)
		.unwrap_or(DEFAULT_PADDING)
}

fn identifier_from_settings(settings: Option<&Value>) -> String {
	setting_string(settings, "case_number_identifier")
		.or_else(|| setting_string(settings, "case_number_prefix"))
		.unwrap_or_else(|| DEFAULT_IDENTIFIER.to_string())
}

pub async fn generate_case_number(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<GeneratedCaseNumber> {
	let settings = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY).await?;
	let identifier = identifier_from_settings(settings.as_ref());
	let padding = setting_padding(settings.as_ref());

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	let (count,) = match dbx
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM safety_report_identification WHERE safety_report_id LIKE $1",
			)
			.bind(format!("{identifier}%")),
		)
		.await
	{
		Ok(row) => row,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(crate::model::Error::Store(err.to_string()));
		}
	};
	dbx.commit_txn().await?;

	let mut sequence = count + 1;
	loop {
		let safety_report_id = format!("{identifier}{sequence:0padding$}");
		if !case_number_exists(ctx, mm, &safety_report_id).await? {
			return Ok(GeneratedCaseNumber {
				worldwide_unique_id: safety_report_id.clone(),
				safety_report_id,
			});
		}
		sequence += 1;
	}
}

async fn case_number_exists(
	ctx: &Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<bool> {
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}
	let (exists,) = match dbx
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT EXISTS (SELECT 1 FROM safety_report_identification WHERE safety_report_id = $1)",
			)
			.bind(safety_report_id),
		)
		.await
	{
		Ok(row) => row,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(crate::model::Error::Store(err.to_string()));
		}
	};
	dbx.commit_txn().await?;
	Ok(exists)
}
