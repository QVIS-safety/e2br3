use super::*;

#[derive(Clone, Copy)]
enum CiomsCaseTable {
	SafetyReportIdentification,
	PatientInformation,
	NarrativeInformation,
	PrimarySources,
	SenderInformation,
}

impl CiomsCaseTable {
	fn as_str(self) -> &'static str {
		match self {
			Self::SafetyReportIdentification => "safety_report_identification",
			Self::PatientInformation => "patient_information",
			Self::NarrativeInformation => "narrative_information",
			Self::PrimarySources => "primary_sources",
			Self::SenderInformation => "sender_information",
		}
	}
}

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

async fn load_optional_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: CiomsCaseTable,
	case_id: Uuid,
) -> Result<Option<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let table = table.as_str();
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 LIMIT 1");
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_optional(sqlx::query_as::<_, T>(&sql).bind(case_id))
				.await
				.map_err(ModelError::Dbx)
				.map_err(Error::Model)
		})
	})
	.await
}

async fn load_list_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: CiomsCaseTable,
	case_id: Uuid,
) -> Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let table = table.as_str();
	let sql = format!(
		"SELECT * FROM {table} WHERE case_id = $1 AND deleted IS NOT TRUE ORDER BY sequence_number"
	);
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
				.await
				.map_err(ModelError::Dbx)
				.map_err(Error::Model)
		})
	})
	.await
}

async fn load_unordered_list_by_case<T>(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: CiomsCaseTable,
	case_id: Uuid,
) -> Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let table = table.as_str();
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 ORDER BY id");
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
				.await
				.map_err(ModelError::Dbx)
				.map_err(Error::Model)
		})
	})
	.await
}

pub(super) async fn load_dosages_by_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DosageInformation>> {
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(
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
			.map_err(ModelError::Dbx)
			.map_err(Error::Model)
		})
	})
	.await
}

pub(super) async fn load_indications_by_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DrugIndication>> {
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(
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
			.map_err(ModelError::Dbx)
			.map_err(Error::Model)
		})
	})
	.await
}

pub(super) async fn load_cioms_case_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CiomsCaseData> {
	let report = load_optional_by_case::<SafetyReportIdentification>(
		ctx,
		mm,
		CiomsCaseTable::SafetyReportIdentification,
		case_id,
	)
	.await?;
	let case_number = report
		.as_ref()
		.and_then(|report| report.safety_report_id.clone())
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| case_id.to_string());
	let patient = load_optional_by_case::<PatientInformation>(
		ctx,
		mm,
		CiomsCaseTable::PatientInformation,
		case_id,
	)
	.await?;
	let narrative = load_optional_by_case::<NarrativeInformation>(
		ctx,
		mm,
		CiomsCaseTable::NarrativeInformation,
		case_id,
	)
	.await?;
	let reactions = ReactionBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let drugs = DrugInformationBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let dosages = load_dosages_by_case(ctx, mm, case_id).await?;
	let indications = load_indications_by_case(ctx, mm, case_id).await?;
	let primary_sources = load_list_by_case::<PrimarySource>(
		ctx,
		mm,
		CiomsCaseTable::PrimarySources,
		case_id,
	)
	.await?;
	let senders = load_unordered_list_by_case::<SenderInformation>(
		ctx,
		mm,
		CiomsCaseTable::SenderInformation,
		case_id,
	)
	.await?;
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
