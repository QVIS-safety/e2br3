use crate::common::{demo_ctx, init_test_mm, system_user_id, unique_suffix, Result};
use lib_core::model::store::set_full_context_dbx_or_rollback;
use lib_core::model::terminology::{
	ControlledTermBmc, E2bCodeListBmc, FdaHierarchicalCodeListBmc, IsoCountryBmc,
	MeddraTermBmc, MeddraTermKey, MfdsProductBmc, UcumUnitBmc, WhodrugProductBmc,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn bootstrap_provides_active_iso_country_reference_data() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();

	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		system_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;
	dbx.execute(sqlx::query("DELETE FROM iso_countries"))
		.await?;
	dbx.execute(sqlx::query(include_str!(
		"../../../../../db/bootstrap/09a-iso-countries.sql"
	)))
	.await?;

	let countries = IsoCountryBmc::list_all(&ctx, &mm).await?;
	assert!(
		countries.len() >= 200,
		"fresh database must provide a usable ISO country catalog"
	);
	assert!(countries.iter().any(|country| country.code == "KR"));
	assert!(countries.iter().any(|country| country.code == "US"));

	dbx.rollback_txn().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn active_controlled_terms_and_mfds_products_support_membership() -> Result<()>
{
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();
	let suffix = unique_suffix();
	let version = format!("test-{}", &suffix[..16]);
	let whodrug_version = format!("T{}", &suffix[..8]);
	let item_seq = format!("P{}", &suffix[..8]);
	let substance_code = format!("S{}", &suffix[..8]);
	let whodrug_code = format!("W{}", &suffix[..8]);

	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		system_user_id(),
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
	dbx.execute(sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='whodrug' AND language='en' AND status='active'")).await?;
	dbx.execute(sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows) VALUES ('whodrug',$1,'en','active',1)").bind(&whodrug_version)).await?;

	dbx.execute(
		sqlx::query(
			"INSERT INTO whodrug_products
			 (code, drug_name, version, language, active)
			 VALUES ($1, 'Test WHODrug product', $2, 'en', true)",
		)
		.bind(&whodrug_code)
		.bind(&whodrug_version),
	)
	.await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO terminology_releases
			 (dictionary, version, language, status, loaded_rows)
			 VALUES ('edqm', $1, 'en', 'active', 1)",
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
	dbx.execute(
		sqlx::query(
			"INSERT INTO mfds_product_substances
			 (item_seq, substance_code, substance_name_kr, version, active)
			 VALUES ($1, $2, '시험 성분', $3, true)",
		)
		.bind(&item_seq)
		.bind(&substance_code)
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
	let edqm_versions =
		ControlledTermBmc::active_release_versions(&mm, "edqm", "en").await?;
	assert!(edqm_versions.contains(&version));

	let product_codes = vec![item_seq.clone(), "missing".to_string()];
	let existing_products =
		MfdsProductBmc::existing_active_item_seqs(&mm, &product_codes).await?;
	assert!(existing_products.contains(&item_seq));
	assert!(!existing_products.contains("missing"));
	let existing_substances = MfdsProductBmc::existing_active_substance_codes(
		&mm,
		&[substance_code.clone(), "missing".to_string()],
	)
	.await?;
	assert!(existing_substances.contains(&substance_code));
	assert!(!existing_substances.contains("missing"));
	let whodrug_codes = vec![whodrug_code.clone(), "missing".to_string()];
	let existing_whodrug =
		WhodrugProductBmc::existing_active_codes(&mm, &whodrug_codes).await?;
	assert!(existing_whodrug.contains(&whodrug_code));
	assert!(!existing_whodrug.contains("missing"));
	assert!(WhodrugProductBmc::active_versions(&mm)
		.await?
		.contains(&whodrug_version));
	let existing_whodrug_keys = WhodrugProductBmc::existing_active_keys(
		&mm,
		&[
			(whodrug_version.clone(), whodrug_code.clone()),
			(whodrug_version.clone(), "missing".to_string()),
		],
	)
	.await?;
	assert!(existing_whodrug_keys
		.contains(&(whodrug_version.clone(), whodrug_code.clone())));

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
		system_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;

	dbx.execute(sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='meddra' AND language='en' AND status='active'")).await?;
	dbx.execute(sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows) VALUES ('meddra',$1,'en','active',2)").bind(&meddra_version)).await?;

	if let Err(err) = dbx
		.execute(
			sqlx::query(
				"INSERT INTO meddra_terms (code, term, level, version, language)
		 VALUES ($1, $2, $3, $4, $5)",
			)
			.bind(&meddra_code)
			.bind(&meddra_term)
			.bind("LLT")
			.bind(&meddra_version)
			.bind("en"),
		)
		.await
	{
		dbx.rollback_txn().await?;
		return Err(err.into());
	}
	let hlt_code = format!("HLT{}", &suffix[..8]);
	dbx.execute(
		sqlx::query(
			"INSERT INTO meddra_terms (code, term, level, version, language)
		 VALUES ($1, $2, 'HLT', $3, 'en')",
		)
		.bind(&hlt_code)
		.bind(format!("{meddra_term} HLT"))
		.bind(&meddra_version),
	)
	.await?;

	dbx.execute(sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='whodrug' AND language='en' AND status='active'")).await?;
	dbx.execute(sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows) VALUES ('whodrug',$1,'en','active',1)").bind(&meddra_version)).await?;

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
		None,
		5,
	)
	.await?;
	assert!(meddra_terms.iter().any(|t| t.code == meddra_code));
	let llt_terms = MeddraTermBmc::search(
		&ctx,
		&mm,
		"TestTerm",
		Some(&meddra_version),
		Some("en"),
		Some("LLT"),
		10,
	)
	.await?;
	assert!(llt_terms.iter().any(|term| term.code == meddra_code));
	assert!(llt_terms.iter().all(|term| term.code != hlt_code));
	let active_keys = MeddraTermBmc::existing_active_keys(
		&mm,
		&[
			MeddraTermKey {
				version: meddra_version.clone(),
				code: meddra_code.clone(),
			},
			MeddraTermKey {
				version: meddra_version.clone(),
				code: hlt_code,
			},
		],
	)
	.await?;
	assert_eq!(active_keys.len(), 1);
	assert_eq!(active_keys[0].code, meddra_code);

	let whodrug = WhodrugProductBmc::search(&ctx, &mm, &whodrug_name, 50).await?;
	assert!(whodrug.iter().any(|p| p.code == whodrug_code));

	let countries = IsoCountryBmc::list_all(&ctx, &mm).await?;
	assert!(countries.iter().any(|c| c.code == iso_code));

	let report_types =
		E2bCodeListBmc::get_by_list_name(&ctx, &mm, "report_type").await?;
	assert!(!report_types.is_empty());

	let ucum_units = UcumUnitBmc::list_all(&ctx, &mm).await?;
	assert!(ucum_units.len() >= 31);
	assert!(ucum_units.iter().any(|u| {
		u.code == "ug"
			&& u.display_name == "microgram"
			&& u.unit_type.as_deref() == Some("mass")
	}));
	assert!(ucum_units.iter().any(|u| {
		u.code == "[IU]"
			&& u.display_name == "international unit"
			&& u.unit_type.as_deref() == Some("dose")
	}));

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

#[serial]
#[tokio::test]
async fn whodrug_release_switch_preserves_rows_and_audits_transition() -> Result<()>
{
	use lib_core::model::terminology_import::activate_release_tx;
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();
	let suffix = unique_suffix();
	let old = format!("old{}", &suffix[..8]);
	let new = format!("new{}", &suffix[..8]);
	let name = format!("ReleaseSwitch{suffix}");
	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		system_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;
	for (version, status) in [(&old, "active"), (&new, "approved")] {
		dbx.execute(sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows,source_checksum,approved_by,approved_at) VALUES ('whodrug',$1,'zz',$2,1,'fixture-hash',$3,NOW())").bind(version).bind(status).bind(system_user_id())).await?;
		dbx.execute(sqlx::query("INSERT INTO whodrug_products(code,drug_name,version,language,active) VALUES ('1234567',$1,$2,'zz',false)").bind(&name).bind(version)).await?;
		dbx.execute(sqlx::query("INSERT INTO controlled_terminology_terms(dictionary,version,language,scope,code,active) VALUES ('whodrug',$1,'zz','cas','0000103902',false)").bind(version)).await?;
	}
	let before: (String,) = dbx.fetch_one(sqlx::query_as("SELECT json_agg(to_jsonb(p) ORDER BY version)::text FROM whodrug_products p WHERE language='zz' AND version IN ($1,$2)").bind(&old).bind(&new)).await?;
	dbx.commit_txn().await?;
	for (target, previous, rollback) in [(&new, &old, false), (&old, &new, true)] {
		let release = activate_release_tx(
			&mm,
			system_user_id(),
			"whodrug",
			target,
			"zz",
			rollback,
		)
		.await?;
		assert_eq!(release.status, "active");
		if rollback {
			assert_eq!(
				release.rollback_from_version.as_deref(),
				Some(previous.as_str())
			);
		}
		dbx.begin_txn().await?;
		set_full_context_dbx_or_rollback(
			dbx,
			system_user_id(),
			ctx.organization_id(),
			"system_admin",
		)
		.await?;
		set_full_context_dbx_or_rollback(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		dbx.execute(sqlx::query("SET LOCAL ROLE e2br3_app_role"))
			.await?;
		let visible: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM whodrug_products WHERE language='zz' AND version=$1").bind(previous)).await?;
		assert_eq!(
			visible.0, 0,
			"retired release must stay invisible to reference readers"
		);

		let keys = vec![
			(old.clone(), "1234567".to_string()),
			(new.clone(), "1234567".to_string()),
		];
		assert_eq!(
			WhodrugProductBmc::existing_active_keys(&mm, &keys).await?,
			[(target.clone(), "1234567".to_string())]
				.into_iter()
				.collect()
		);
		let found = WhodrugProductBmc::search(&ctx, &mm, &name, 20).await?;
		assert_eq!(found.len(), 1);
		assert_eq!(&found[0].version, target);
		assert!(found[0].active);
		let cas = vec![
			(old.clone(), "0000103902".to_string()),
			(new.clone(), "0000103902".to_string()),
		];
		assert_eq!(
			ControlledTermBmc::existing_active_keys(&mm, "whodrug", "cas", &cas)
				.await?,
			[(target.clone(), "0000103902".to_string())]
				.into_iter()
				.collect()
		);
		dbx.execute(sqlx::query("RESET ROLE")).await?;
		set_full_context_dbx_or_rollback(
			dbx,
			system_user_id(),
			ctx.organization_id(),
			"system_admin",
		)
		.await?;

		let after: (String,) = dbx.fetch_one(sqlx::query_as("SELECT json_agg(to_jsonb(p) ORDER BY version)::text FROM whodrug_products p WHERE language='zz' AND version IN ($1,$2)").bind(&old).bind(&new)).await?;
		assert_eq!(before, after);
		let audit: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM audit_logs a JOIN terminology_releases r ON a.record_id=r.audit_id WHERE a.table_name='terminology_releases' AND r.version=$1 AND a.action='UPDATE' AND a.new_values->>'status'='active'").bind(target)).await?;
		assert_eq!(audit.0, 1);
		let updates: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM audit_logs WHERE table_name IN ('whodrug_products','controlled_terminology_terms') AND action='UPDATE' AND new_values->>'version' IN ($1,$2)").bind(&old).bind(&new)).await?;
		assert_eq!(updates.0, 0);
		dbx.commit_txn().await?;
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn meddra_release_switch_preserves_rows_and_audits_transition() -> Result<()> {
	use lib_core::model::terminology_import::activate_release_tx;
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let dbx = mm.dbx();
	let suffix = unique_suffix();
	let old = format!("old{}", &suffix[..6]);
	let new = format!("new{}", &suffix[..6]);
	let name = format!("ReleaseSwitch{suffix}");
	dbx.begin_txn().await?;
	set_full_context_dbx_or_rollback(
		dbx,
		system_user_id(),
		ctx.organization_id(),
		"system_admin",
	)
	.await?;
	for (version, status) in [(&old, "active"), (&new, "approved")] {
		dbx.execute(sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows,source_checksum,approved_by,approved_at) VALUES ('meddra',$1,'zz',$2,1,'fixture-hash',$3,NOW())").bind(version).bind(status).bind(system_user_id())).await?;
		dbx.execute(sqlx::query("INSERT INTO meddra_terms(code,term,level,version,language,active) VALUES ('1234567',$1,'LLT',$2,'zz',false)").bind(&name).bind(version)).await?;
	}
	let before: (String,) = dbx.fetch_one(sqlx::query_as("SELECT json_agg(to_jsonb(p) ORDER BY version)::text FROM meddra_terms p WHERE language='zz' AND version IN ($1,$2)").bind(&old).bind(&new)).await?;
	dbx.commit_txn().await?;
	for (target, previous, rollback) in [(&new, &old, false), (&old, &new, true)] {
		let release = activate_release_tx(
			&mm,
			system_user_id(),
			"meddra",
			target,
			"zz",
			rollback,
		)
		.await?;
		assert_eq!(release.status, "active");
		if rollback {
			assert_eq!(
				release.rollback_from_version.as_deref(),
				Some(previous.as_str())
			);
		}
		dbx.begin_txn().await?;
		set_full_context_dbx_or_rollback(
			dbx,
			system_user_id(),
			ctx.organization_id(),
			"system_admin",
		)
		.await?;
		set_full_context_dbx_or_rollback(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		dbx.execute(sqlx::query("SET LOCAL ROLE e2br3_app_role"))
			.await?;
		let visible: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM meddra_terms WHERE language='zz' AND version=$1").bind(previous)).await?;
		assert_eq!(
			visible.0, 0,
			"retired release must stay invisible to reference readers"
		);

		let keys = vec![
			MeddraTermKey {
				version: old.clone(),
				code: "1234567".into(),
			},
			MeddraTermKey {
				version: new.clone(),
				code: "1234567".into(),
			},
		];
		assert_eq!(
			MeddraTermBmc::existing_active_keys(&mm, &keys).await?,
			vec![MeddraTermKey {
				version: target.clone(),
				code: "1234567".into()
			}]
		);

		let found = MeddraTermBmc::search(
			&ctx,
			&mm,
			&name,
			Some(target),
			Some("zz"),
			Some("LLT"),
			20,
		)
		.await?;
		assert_eq!(found.len(), 1);
		assert_eq!(&found[0].version, target);
		assert!(found[0].active);
		dbx.execute(sqlx::query("RESET ROLE")).await?;
		set_full_context_dbx_or_rollback(
			dbx,
			system_user_id(),
			ctx.organization_id(),
			"system_admin",
		)
		.await?;

		let after: (String,) = dbx.fetch_one(sqlx::query_as("SELECT json_agg(to_jsonb(p) ORDER BY version)::text FROM meddra_terms p WHERE language='zz' AND version IN ($1,$2)").bind(&old).bind(&new)).await?;
		assert_eq!(before, after);
		let audit: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM audit_logs a JOIN terminology_releases r ON a.record_id=r.audit_id WHERE a.table_name='terminology_releases' AND r.version=$1 AND a.action='UPDATE' AND a.new_values->>'status'='active'").bind(target)).await?;
		assert_eq!(audit.0, 1);
		let updates: (i64,) = dbx.fetch_one(sqlx::query_as("SELECT count(*) FROM audit_logs WHERE table_name IN ('meddra_terms','controlled_terminology_terms') AND action='UPDATE' AND new_values->>'version' IN ($1,$2)").bind(&old).bind(&new)).await?;
		assert_eq!(updates.0, 0);
		dbx.commit_txn().await?;
	}
	Ok(())
}
