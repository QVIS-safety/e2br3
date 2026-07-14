use crate::common::{demo_ctx, demo_user_id, init_test_mm, unique_suffix, Result};
use lib_core::model::store::set_full_context_dbx_or_rollback;
use lib_core::model::terminology::{
	ControlledTermBmc, E2bCodeListBmc, FdaHierarchicalCodeListBmc, IsoCountryBmc,
	MeddraTermBmc, MfdsProductBmc, UcumUnitBmc, WhodrugProductBmc,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn active_controlled_terms_and_mfds_products_support_membership() -> Result<()>
{
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();
	let suffix = unique_suffix();
	let version = format!("test-{}", &suffix[..16]);
	let item_seq = format!("P{}", &suffix[..8]);

	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		demo_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;

	dbx.execute(
		sqlx::query(
			"INSERT INTO controlled_terminology_terms
			 (dictionary, version, language, scope, code, display_name, active)
			 VALUES ('iso3166', $1, 'en', 'country', 'KR', 'Korea', true)",
		)
		.bind(&version),
	)
	.await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO mfds_products
			 (item_seq, product_name_kr, version, active)
			 VALUES ($1, 'Test product', $2, true)",
		)
		.bind(&item_seq)
		.bind(&version),
	)
	.await?;

	let country_codes = vec!["KR".to_string(), "ZZ".to_string()];
	let existing_countries = ControlledTermBmc::existing_active_codes(
		&mm,
		"iso3166",
		"country",
		&country_codes,
	)
	.await?;
	assert!(existing_countries.contains("KR"));
	assert!(!existing_countries.contains("ZZ"));

	let product_codes = vec![item_seq.clone(), "missing".to_string()];
	let existing_products =
		MfdsProductBmc::existing_active_item_seqs(&mm, &product_codes).await?;
	assert!(existing_products.contains(&item_seq));
	assert!(!existing_products.contains("missing"));

	dbx.rollback_txn().await?;
	Ok(())
}

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

#[serial]
#[tokio::test]
async fn test_fda_hierarchical_code_list_search() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	// Seeded by db/bootstrap/11-fda-device-codes.sql — real Device Problem Code data.
	let results = FdaHierarchicalCodeListBmc::search(
		&ctx,
		&mm,
		"device_problem_code",
		"Biocompatibility",
		10,
	)
	.await?;

	assert!(results.iter().any(|r| r.fda_code == "2886"
		&& r.imdrf_code == "IMDRF:A010101"
		&& r.level1_term == "Patient Device Interaction Problem"
		&& r.level2_term.as_deref() == Some("Patient-Device Incompatibility")
		&& r.level3_term.as_deref() == Some("Biocompatibility")));

	let no_match = FdaHierarchicalCodeListBmc::search(
		&ctx,
		&mm,
		"device_problem_code",
		"ZzNoSuchTermZz",
		10,
	)
	.await?;
	assert!(no_match.is_empty());

	Ok(())
}
