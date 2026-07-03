use super::*;

pub(super) async fn load_cioms_settings(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
) -> Result<CiomsSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	let orientation = value
		.as_ref()
		.and_then(|value| value.get("orientation"))
		.and_then(|value| value.as_str())
		.unwrap_or("Landscape")
		.trim()
		.to_string();
	let data_ordering = value
		.as_ref()
		.and_then(|value| value.get("data_ordering"))
		.and_then(|value| value.as_str())
		.unwrap_or("Primary data will appear first")
		.trim()
		.to_string();
	Ok(CiomsSettings {
		orientation: if orientation.eq_ignore_ascii_case("portrait") {
			"Portrait".to_string()
		} else {
			"Landscape".to_string()
		},
		data_ordering,
	})
}

pub(super) async fn load_optional_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Option<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err);
	}
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 LIMIT 1");
	let result = mm
		.dbx()
		.fetch_optional(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx);
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) async fn load_list_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err);
	}
	let sql = format!(
		"SELECT * FROM {table} WHERE case_id = $1 AND deleted IS NOT TRUE ORDER BY sequence_number"
	);
	let result = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx);
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) async fn load_unordered_list_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err);
	}
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 ORDER BY id");
	let result = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx);
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) async fn load_dosages_by_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<DosageInformation>> {
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err);
	}
	let result = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, DosageInformation>(
				"SELECT dosage_information.*
				 FROM dosage_information
				 JOIN drug_information ON drug_information.id = dosage_information.drug_id
				 WHERE drug_information.case_id = $1
				   AND drug_information.deleted IS NOT TRUE
				   AND dosage_information.deleted IS NOT TRUE
				 ORDER BY drug_information.sequence_number, dosage_information.sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(ModelError::Dbx);
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) async fn load_indications_by_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<DrugIndication>> {
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err);
	}
	let result = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, DrugIndication>(
				"SELECT drug_indications.*
				 FROM drug_indications
				 JOIN drug_information ON drug_information.id = drug_indications.drug_id
				 WHERE drug_information.case_id = $1
				   AND drug_information.deleted IS NOT TRUE
				   AND drug_indications.deleted IS NOT TRUE
				 ORDER BY drug_information.sequence_number, drug_indications.sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(ModelError::Dbx);
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) async fn load_cioms_case_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CiomsCaseData> {
	let report = load_optional_by_case::<SafetyReportIdentification>(
		ctx,
		mm,
		"safety_report_identification",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let case_number = report
		.as_ref()
		.and_then(|report| report.safety_report_id.clone())
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| case_id.to_string());
	let patient = load_optional_by_case::<PatientInformation>(
		ctx,
		mm,
		"patient_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let narrative = load_optional_by_case::<NarrativeInformation>(
		ctx,
		mm,
		"narrative_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let reactions = ReactionBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let drugs = DrugInformationBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let dosages = load_dosages_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let indications = load_indications_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let primary_sources =
		load_list_by_case::<PrimarySource>(ctx, mm, "primary_sources", case_id)
			.await
			.map_err(Error::Model)?;
	let senders = load_unordered_list_by_case::<SenderInformation>(
		ctx,
		mm,
		"sender_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	Ok(CiomsCaseData {
		case_number,
		report,
		patient,
		reactions,
		drugs,
		dosages,
		indications,
		primary_sources,
		senders,
		narrative,
	})
}
