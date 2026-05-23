mod common;

use crate::common::Result;
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn section_presave_tables_exist() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let tables = [
		"sender_presaves",
		"sender_presave_gateways",
		"sender_presave_responsible_persons",
		"receiver_presaves",
		"receiver_presave_consignees",
		"product_presaves",
		"product_presave_substances",
		"product_presave_fda_cross_reported_inds",
		"product_presave_mfds_regional_items",
		"reporter_presaves",
		"study_presaves",
		"study_presave_registration_numbers",
		"narrative_presaves",
		"narrative_presave_sender_diagnoses",
		"narrative_presave_case_summaries",
	];

	for table in tables {
		let exists: bool = sqlx::query_scalar(
			"SELECT EXISTS (
				SELECT 1 FROM information_schema.tables
				WHERE table_schema = 'public' AND table_name = $1
			)",
		)
		.bind(table)
		.fetch_one(mm.dbx().db())
		.await?;
		assert!(exists, "missing table {table}");
	}

	Ok(())
}
