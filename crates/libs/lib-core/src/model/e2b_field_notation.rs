use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{Error, ModelManager, Result};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct E2bFieldNotation {
	pub id: Uuid,
	pub record_id: Option<Uuid>,
	pub e2b_code: String,
	pub notation: String,
}

pub struct E2bFieldNotationBmc;

impl E2bFieldNotationBmc {
	async fn ensure_record_belongs_to_case(
		mm: &ModelManager,
		case_id: Uuid,
		record_id: Option<Uuid>,
	) -> Result<()> {
		let Some(record_id) = record_id else {
			return Ok(());
		};
		// Registry editor audit subjects plus the frontend-only device-characteristic
		// subject; deleted editor rows retain their identity for read/restore.
		let (belongs_to_case,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (bool,)>(
					"WITH record_cases(case_id) AS (
						 SELECT id FROM cases WHERE id = $2
						 UNION ALL SELECT case_id FROM safety_report_identification WHERE id = $2
						 UNION ALL SELECT case_id FROM documents_held_by_sender WHERE id = $2
						 UNION ALL SELECT case_id FROM source_documents WHERE id = $2
						 UNION ALL SELECT case_id FROM other_case_identifiers WHERE id = $2
						 UNION ALL SELECT case_id FROM linked_report_numbers WHERE id = $2
						 UNION ALL SELECT case_id FROM primary_sources WHERE id = $2
						 UNION ALL SELECT case_id FROM sender_information WHERE id = $2
						 UNION ALL SELECT case_id FROM receiver_information WHERE id = $2
						 UNION ALL SELECT case_id FROM literature_references WHERE id = $2
						 UNION ALL SELECT case_id FROM study_information WHERE id = $2
						 UNION ALL
						 SELECT si.case_id
						 FROM study_registration_numbers subject
						 JOIN study_information si ON si.id = subject.study_information_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT si.case_id
						 FROM study_fda_cross_reported_inds subject
						 JOIN study_information si ON si.id = subject.study_information_id
						 WHERE subject.id = $2
						 UNION ALL SELECT case_id FROM patient_information WHERE id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM patient_identifiers subject
						 JOIN patient_information pi ON pi.id = subject.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM medical_history_episodes subject
						 JOIN patient_information pi ON pi.id = subject.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM past_drug_history subject
						 JOIN patient_information pi ON pi.id = subject.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM patient_death_information subject
						 JOIN patient_information pi ON pi.id = subject.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM reported_causes_of_death subject
						 JOIN patient_death_information pdi ON pdi.id = subject.death_info_id
						 JOIN patient_information pi ON pi.id = pdi.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM autopsy_causes_of_death subject
						 JOIN patient_death_information pdi ON pdi.id = subject.death_info_id
						 JOIN patient_information pi ON pi.id = pdi.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM parent_information subject
						 JOIN patient_information pi ON pi.id = subject.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM parent_medical_history subject
						 JOIN parent_information parent ON parent.id = subject.parent_id
						 JOIN patient_information pi ON pi.id = parent.patient_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT pi.case_id
						 FROM parent_past_drug_history subject
						 JOIN parent_information parent ON parent.id = subject.parent_id
						 JOIN patient_information pi ON pi.id = parent.patient_id
						 WHERE subject.id = $2
						 UNION ALL SELECT case_id FROM reactions WHERE id = $2
						 UNION ALL SELECT case_id FROM test_results WHERE id = $2
						 UNION ALL SELECT case_id FROM drug_information WHERE id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM drug_active_substances subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM dosage_information subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM drug_indications subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM drug_device_characteristics subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM fda_device_information subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM fda_device_codes subject
						 JOIN fda_device_information fdi ON fdi.id = subject.device_id
						 JOIN drug_information di ON di.id = fdi.drug_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM drug_reaction_assessments subject
						 JOIN drug_information di ON di.id = subject.drug_id
						 JOIN reactions reaction
						   ON reaction.id = subject.reaction_id AND reaction.case_id = di.case_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT di.case_id
						 FROM relatedness_assessments subject
						 JOIN drug_reaction_assessments dra
						   ON dra.id = subject.drug_reaction_assessment_id
						 JOIN drug_information di ON di.id = dra.drug_id
						 JOIN reactions reaction
						   ON reaction.id = dra.reaction_id AND reaction.case_id = di.case_id
						 WHERE subject.id = $2
						 UNION ALL SELECT case_id FROM narrative_information WHERE id = $2
						 UNION ALL
						 SELECT ni.case_id
						 FROM sender_diagnoses subject
						 JOIN narrative_information ni ON ni.id = subject.narrative_id
						 WHERE subject.id = $2
						 UNION ALL
						 SELECT ni.case_id
						 FROM case_summary_information subject
						 JOIN narrative_information ni ON ni.id = subject.narrative_id
						 WHERE subject.id = $2
					 )
					 SELECT EXISTS (
						 SELECT 1 FROM record_cases WHERE case_id = $1
					 )",
				)
				.bind(case_id)
				.bind(record_id),
			)
			.await?;
		if !belongs_to_case {
			return Err(Error::EntityUuidNotFound {
				entity: "E2B field notation record",
				id: record_id,
			});
		}
		Ok(())
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		record_id: Option<Uuid>,
		e2b_code: &str,
	) -> Result<Option<E2bFieldNotation>> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) =
			Self::ensure_record_belongs_to_case(mm, case_id, record_id).await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, E2bFieldNotation>(
					"SELECT id, record_id, e2b_code, notation
					 FROM case_e2b_field_notations
					 WHERE case_id = $1 AND record_id IS NOT DISTINCT FROM $2
					   AND e2b_code = $3",
				)
				.bind(case_id)
				.bind(record_id)
				.bind(e2b_code),
			)
			.await;
		match result {
			Ok(row) => {
				mm.dbx().commit_txn().await?;
				Ok(row)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<E2bFieldNotation>> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let result = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, E2bFieldNotation>(
					"SELECT id, record_id, e2b_code, notation
					 FROM case_e2b_field_notations
					 WHERE case_id = $1 ORDER BY e2b_code, record_id",
				)
				.bind(case_id),
			)
			.await;
		match result {
			Ok(rows) => {
				mm.dbx().commit_txn().await?;
				Ok(rows)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn upsert(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		record_id: Option<Uuid>,
		e2b_code: &str,
		notation: &str,
	) -> Result<E2bFieldNotation> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) =
			Self::ensure_record_belongs_to_case(mm, case_id, record_id).await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let result = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, E2bFieldNotation>(
					"INSERT INTO case_e2b_field_notations
					 (case_id, record_id, e2b_code, notation, created_by, updated_by)
					 VALUES ($1, $2, $3, $4, $5, $5)
					 ON CONFLICT (case_id, record_id, e2b_code)
					 DO UPDATE SET notation = EXCLUDED.notation, updated_by = EXCLUDED.updated_by
					 RETURNING id, record_id, e2b_code, notation",
				)
				.bind(case_id)
				.bind(record_id)
				.bind(e2b_code)
				.bind(notation)
				.bind(ctx.user_id()),
			)
			.await;
		match result {
			Ok(row) => {
				mm.dbx().commit_txn().await?;
				Ok(row)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn delete(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		record_id: Option<Uuid>,
		e2b_code: &str,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) =
			Self::ensure_record_belongs_to_case(mm, case_id, record_id).await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let result = mm
			.dbx()
			.execute(
				sqlx::query(
					"DELETE FROM case_e2b_field_notations
					 WHERE case_id = $1 AND record_id IS NOT DISTINCT FROM $2
					   AND e2b_code = $3",
				)
				.bind(case_id)
				.bind(record_id)
				.bind(e2b_code),
			)
			.await;
		match result {
			Ok(_) => {
				mm.dbx().commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}
}
