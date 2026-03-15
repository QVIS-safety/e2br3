use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{self, ModelManager};
use crate::xml::import_runtime::shared;
use crate::xml::{error::Error, Result};
use sqlx::types::Uuid;

pub(crate) async fn import_section_c(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&shared::MessageHeaderExtract>,
) -> Result<()> {
	import_c_1_safety_report(ctx, mm, xml, case_id, header).await?;
	shared::import_sender_information(ctx, mm, xml, case_id, header).await?;
	shared::import_primary_sources(ctx, mm, xml, case_id).await?;
	shared::import_case_identifiers(ctx, mm, xml, case_id).await?;
	shared::import_documents_held_by_sender(ctx, mm, xml, case_id).await?;
	shared::import_literature_references(ctx, mm, xml, case_id).await?;
	import_c_5_study_information(ctx, mm, xml, case_id).await?;
	shared::import_receiver_information(ctx, mm, xml, case_id).await?;
	Ok(())
}

async fn import_c_1_safety_report(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&shared::MessageHeaderExtract>,
) -> Result<()> {
	let Some(report) =
		crate::xml::import_sections::c_safety_report::parse_c_safety_report(xml)?
	else {
		return Ok(());
	};

	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(err));
	}

	let receiver_organization = header.and_then(|h| h.message_receiver.clone());

	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO safety_report_identification (
					case_id,
					transmission_date,
					transmission_date_null_flavor,
					report_type,
					date_first_received_from_source,
					date_first_received_from_source_null_flavor,
					date_of_most_recent_information,
					date_of_most_recent_information_null_flavor,
					fulfil_expedited_criteria,
					local_criteria_report_type,
					combination_product_report_indicator,
					worldwide_unique_id,
					first_sender_type,
					additional_documents_available,
					nullification_code,
					nullification_reason,
					receiver_organization,
					created_at,
					updated_at,
					created_by
				) VALUES (
					$1,$2,NULL,$3,$4,NULL,$5,NULL,$6,$7,$8,$9,$10,$11,$12,$13,$14,NOW(),NOW(),$15
				)
				ON CONFLICT (case_id) DO UPDATE SET
					transmission_date = EXCLUDED.transmission_date,
					transmission_date_null_flavor = NULL,
					report_type = EXCLUDED.report_type,
					date_first_received_from_source = EXCLUDED.date_first_received_from_source,
					date_first_received_from_source_null_flavor = NULL,
					date_of_most_recent_information = EXCLUDED.date_of_most_recent_information,
					date_of_most_recent_information_null_flavor = NULL,
					fulfil_expedited_criteria = EXCLUDED.fulfil_expedited_criteria,
					local_criteria_report_type = EXCLUDED.local_criteria_report_type,
					combination_product_report_indicator = EXCLUDED.combination_product_report_indicator,
					worldwide_unique_id = EXCLUDED.worldwide_unique_id,
					first_sender_type = EXCLUDED.first_sender_type,
					additional_documents_available = EXCLUDED.additional_documents_available,
					nullification_code = EXCLUDED.nullification_code,
					nullification_reason = EXCLUDED.nullification_reason,
					receiver_organization = EXCLUDED.receiver_organization,
					updated_at = NOW(),
					updated_by = $15",
			)
			.bind(case_id)
			.bind(report.transmission_date)
			.bind(report.report_type)
			.bind(report.date_first_received_from_source)
			.bind(report.date_of_most_recent_information)
			.bind(report.fulfil_expedited_criteria)
			.bind(report.local_criteria_report_type)
			.bind(report.combination_product_report_indicator)
			.bind(report.worldwide_unique_id)
			.bind(report.first_sender_type)
			.bind(report.additional_documents_available)
			.bind(report.nullification_code)
			.bind(report.nullification_reason)
			.bind(receiver_organization)
			.bind(ctx.user_id()),
		)
		.await
		.map_err(model::Error::from)?;
	let (visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM safety_report_identification WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?;
	if visible_count != 1 {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(model::Error::Store(format!(
			"section C safety report write invariant failed for case {case_id}: visible_count={visible_count}"
		))));
	}
	mm.dbx().commit_txn().await.map_err(model::Error::from)?;

	Ok(())
}

async fn import_c_5_study_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some(study) = shared::parse_study_information(xml)? else {
		return Ok(());
	};

	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(err));
	}

	let (study_id,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Uuid,)>(
				"INSERT INTO study_information (
					case_id,
					study_name,
					sponsor_study_number,
					study_type_reaction,
					study_type_reaction_kr1,
					created_at,
					updated_at,
					created_by
				) VALUES ($1,$2,$3,$4,NULL,NOW(),NOW(),$5)
				ON CONFLICT (case_id) DO UPDATE SET
					study_name = EXCLUDED.study_name,
					sponsor_study_number = EXCLUDED.sponsor_study_number,
					study_type_reaction = EXCLUDED.study_type_reaction,
					study_type_reaction_kr1 = EXCLUDED.study_type_reaction_kr1,
					updated_at = NOW(),
					updated_by = $5
				RETURNING id",
			)
			.bind(case_id)
			.bind(study.study_name)
			.bind(study.sponsor_study_number)
			.bind(study.study_type_reaction)
			.bind(ctx.user_id()),
		)
		.await
		.map_err(model::Error::from)?;
	let (study_visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM study_information WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?;
	if study_visible_count != 1 {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(model::Error::Store(format!(
			"section C study write invariant failed for case {case_id}: visible_count={study_visible_count}"
		))));
	}

	mm.dbx()
		.execute(
			sqlx::query(
				"DELETE FROM study_registration_numbers WHERE study_information_id = $1",
			)
			.bind(study_id),
		)
		.await
		.map_err(model::Error::from)?;

	for (idx, reg) in study.registrations.into_iter().enumerate() {
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO study_registration_numbers (
						study_information_id,
						registration_number,
						country_code,
						sequence_number,
						created_at,
						updated_at,
						created_by
					) VALUES ($1,$2,$3,$4,NOW(),NOW(),$5)",
				)
				.bind(study_id)
				.bind(reg.registration_number)
				.bind(reg.country_code)
				.bind((idx + 1) as i32)
				.bind(ctx.user_id()),
			)
			.await
			.map_err(model::Error::from)?;
	}
	let (reg_visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM study_registration_numbers WHERE study_information_id = $1",
			)
			.bind(study_id),
		)
		.await
		.map_err(model::Error::from)?;
	if reg_visible_count < 0 {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(model::Error::Store(
			"section C study registration invariant failed".to_string(),
		)));
	}
	mm.dbx().commit_txn().await.map_err(model::Error::from)?;

	Ok(())
}
