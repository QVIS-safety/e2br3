use lib_core::ctx::Ctx;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use lib_core::xml::{import_e2b_xml, XmlImportRequest};
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;

use crate::test_common::{demo_ctx, init_test_mm};

pub struct ImportedCase {
	pub ctx: Ctx,
	pub mm: ModelManager,
	pub case_id: Uuid,
}

pub async fn import_fixture(name: &str) -> ImportedCase {
	import_fixture_with_options(name, Some("fda"), true).await
}

pub async fn import_fixture_with_profile(
	name: &str,
	validation_profile: Option<&str>,
) -> ImportedCase {
	import_fixture_with_options(name, validation_profile, true).await
}

pub async fn import_fixture_with_options(
	name: &str,
	validation_profile: Option<&str>,
	skip_validation: bool,
) -> ImportedCase {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let result = import_e2b_xml(
		&ctx,
		&mm,
		XmlImportRequest {
			xml: fixture(name),
			filename: Some(name.to_string()),
			validation_profile: validation_profile.map(str::to_string),
			skip_validation,
		},
	)
	.await
	.expect("import fixture");
	let case_id =
		Uuid::parse_str(result.case_id.as_deref().expect("case id")).unwrap();
	ImportedCase { ctx, mm, case_id }
}

pub fn fixture(name: &str) -> Vec<u8> {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	std::fs::read(root.join("docs/refs/instances").join(name)).expect("read fixture")
}

pub fn date(year: i32, month: u8, day: u8) -> Date {
	Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
		.expect("valid date")
}

pub fn decimal(value: &str) -> Decimal {
	value.parse::<Decimal>().expect("valid decimal")
}

pub async fn fetch_optional_by_uuid<T>(
	case_: &ImportedCase,
	sql: &str,
	id: Uuid,
) -> Option<T>
where
	T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Unpin + Send,
{
	case_.mm.dbx().begin_txn().await.expect("begin read txn");
	set_full_context_dbx(
		case_.mm.dbx(),
		case_.ctx.user_id(),
		case_.ctx.organization_id(),
		case_.ctx.role(),
	)
	.await
	.expect("set full context");
	let row = case_
		.mm
		.dbx()
		.fetch_optional(sqlx::query_as::<_, T>(sql).bind(id))
		.await
		.expect("fetch optional row");
	case_.mm.dbx().commit_txn().await.expect("commit read txn");
	row
}

pub async fn fetch_one_by_uuid<T>(case_: &ImportedCase, sql: &str, id: Uuid) -> T
where
	T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Unpin + Send,
{
	case_.mm.dbx().begin_txn().await.expect("begin read txn");
	set_full_context_dbx(
		case_.mm.dbx(),
		case_.ctx.user_id(),
		case_.ctx.organization_id(),
		case_.ctx.role(),
	)
	.await
	.expect("set full context");
	let row = case_
		.mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, T>(sql).bind(id))
		.await
		.expect("fetch row");
	case_.mm.dbx().commit_txn().await.expect("commit read txn");
	row
}

pub async fn list_by_uuid<T>(case_: &ImportedCase, sql: &str, id: Uuid) -> Vec<T>
where
	T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Unpin + Send,
{
	case_.mm.dbx().begin_txn().await.expect("begin read txn");
	set_full_context_dbx(
		case_.mm.dbx(),
		case_.ctx.user_id(),
		case_.ctx.organization_id(),
		case_.ctx.role(),
	)
	.await
	.expect("set full context");
	let rows = case_
		.mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, T>(sql).bind(id))
		.await
		.expect("list rows");
	case_.mm.dbx().commit_txn().await.expect("commit read txn");
	rows
}
