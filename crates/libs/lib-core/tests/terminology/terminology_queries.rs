use crate::common::{demo_ctx, demo_user_id, init_test_mm, unique_suffix, Result};
use lib_core::model::store::set_full_context_dbx_or_rollback;
use lib_core::model::terminology::{
	E2bCodeListBmc, IsoCountryBmc, MeddraTermBmc, UcumUnitBmc, WhodrugProductBmc,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_terminology_queries() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();
	let suffix = unique_suffix();
	let meddra_code = format!("MT{}", &suffix[..8]);
	let meddra_term = format!("TestTerm {suffix}");
	let whodrug_code = format!("W{}", &suffix[..8]);
	let whodrug_name = format!("TestDrug-{suffix}");
	let iso_code = format!("Z{}", &suffix[..1]);
	let meddra_version = "v1".to_string();

	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		demo_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;

	if let Err(err) = dbx
		.execute(
			sqlx::query(
				"INSERT INTO meddra_terms (code, term, level, version, language)
		 VALUES ($1, $2, $3, $4, $5)",
			)
			.bind(&meddra_code)
			.bind(&meddra_term)
			.bind("PT")
			.bind(&meddra_version)
			.bind("en"),
		)
		.await
	{
		dbx.rollback_txn().await?;
		return Err(err.into());
	}

	if let Err(err) = dbx
		.execute(
			sqlx::query(
				"INSERT INTO whodrug_products (code, drug_name, atc_code, version, language)
		 VALUES ($1, $2, $3, $4, $5)",
			)
			.bind(&whodrug_code)
			.bind(&whodrug_name)
			.bind("A00")
			.bind(&meddra_version)
			.bind("en"),
		)
		.await
	{
		dbx.rollback_txn().await?;
		return Err(err.into());
	}

	if let Err(err) = dbx
		.execute(
			sqlx::query("DELETE FROM iso_countries WHERE code = $1").bind(&iso_code),
		)
		.await
	{
		dbx.rollback_txn().await?;
		return Err(err.into());
	}

	if let Err(err) = dbx
		.execute(
			sqlx::query(
				"INSERT INTO iso_countries (code, name, active) VALUES ($1, $2, true)",
			)
			.bind(&iso_code)
			.bind("Zedland"),
		)
		.await
	{
		dbx.rollback_txn().await?;
		return Err(err.into());
	}
	// Keep transaction open so session context applies to reads below.

	let meddra_terms = MeddraTermBmc::search(
		&ctx,
		&mm,
		"TestTerm",
		Some(&meddra_version),
		Some("en"),
		5,
	)
	.await?;
	assert!(meddra_terms.iter().any(|t| t.code == meddra_code));

	let whodrug = WhodrugProductBmc::search(&ctx, &mm, &whodrug_name, 50).await?;
	assert!(whodrug.iter().any(|p| p.code == whodrug_code));

	let countries = IsoCountryBmc::list_all(&ctx, &mm).await?;
	assert!(countries.iter().any(|c| c.code == iso_code));

	let report_types =
		E2bCodeListBmc::get_by_list_name(&ctx, &mm, "report_type").await?;
	assert!(!report_types.is_empty());

	let ucum_units = UcumUnitBmc::list_all(&ctx, &mm).await?;
	assert!(ucum_units.iter().any(|u| u.code == "mg/dL"));
	assert!(ucum_units.iter().any(|u| u.code == "U/L"));
	assert!(ucum_units.iter().any(|u| u.code == "mmol/L"));

	dbx.rollback_txn().await?;

	Ok(())
}
