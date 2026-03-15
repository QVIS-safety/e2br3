use crate::ctx::Ctx;
use crate::model;
use crate::model::store::set_full_context_dbx;
use crate::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use crate::model::ModelManager;
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_f(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let tests =
		crate::xml::import_sections::f_test_result::parse_f_test_results(xml)?;

	set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(crate::xml::error::Error::Model)?;

	for (idx, entry) in tests.into_iter().enumerate() {
		let sequence_number = (idx + 1) as i32;
		let existing = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM test_results WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(sequence_number),
			)
			.await
			.map_err(model::Error::from)?
			.map(|row| row.0);

		let update = TestResultForUpdate {
			test_name: Some(entry.test_name.clone()),
			test_date: entry.test_date,
			test_date_null_flavor: entry.test_date_null_flavor,
			test_meddra_version: entry.test_meddra_version,
			test_meddra_code: entry.test_meddra_code,
			test_result_code: entry.test_result_code,
			test_result_value: entry.test_result_value,
			test_result_unit: entry.test_result_unit,
			result_unstructured: entry.result_unstructured,
			normal_low_value: entry.normal_low_value,
			normal_high_value: entry.normal_high_value,
			comments: entry.comments,
			more_info_available: entry.more_info_available,
		};

		if let Some(id) = existing {
			TestResultBmc::update(ctx, mm, id, update).await?;
		} else {
			let id = TestResultBmc::create(
				ctx,
				mm,
				TestResultForCreate {
					case_id,
					sequence_number,
					test_name: entry.test_name,
				},
			)
			.await?;
			TestResultBmc::update(ctx, mm, id, update).await?;
		}
	}

	Ok(())
}
