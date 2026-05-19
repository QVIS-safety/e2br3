use crate::ctx::Ctx;
use crate::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, NarrativeInformationBmc,
	NarrativeInformationForCreate, NarrativeInformationForUpdate,
	SenderDiagnosisBmc, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use crate::model::store::set_full_context_dbx;
use crate::model::{self, ModelManager};
use crate::xml::error::Error;
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_h(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some((narrative, sender_diagnoses, case_summaries)) =
		crate::xml::import_sections::h_narrative::parse_h_narrative(xml)?
			.map(|narrative| {
				Ok::<_, Error>((
					narrative,
					crate::xml::import_sections::h_narrative::parse_h_sender_diagnoses(xml)?,
					crate::xml::import_sections::h_narrative::parse_h_case_summaries(xml)?,
				))
			})
			.transpose()?
	else {
		return Ok(());
	};

	set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(Error::Model)?;

	let narrative_id =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(existing) => {
				NarrativeInformationBmc::update_by_case(
					ctx,
					mm,
					case_id,
					NarrativeInformationForUpdate {
						case_narrative: Some(narrative.case_narrative),
						reporter_comments: narrative.reporter_comments,
						sender_comments: narrative.sender_comments,
					},
				)
				.await?;
				existing.id
			}
			Err(crate::model::Error::EntityUuidNotFound { .. }) => {
				NarrativeInformationBmc::create(
					ctx,
					mm,
					NarrativeInformationForCreate {
						case_id,
						case_narrative: narrative.case_narrative.clone(),
						reporter_comments: narrative.reporter_comments.clone(),
						sender_comments: narrative.sender_comments.clone(),
					},
				)
				.await?
			}
			Err(err) => return Err(err.into()),
		};

	import_sender_diagnoses(ctx, mm, narrative_id, sender_diagnoses).await?;
	import_case_summaries(ctx, mm, narrative_id, case_summaries).await?;
	Ok(())
}

async fn import_sender_diagnoses(
	ctx: &Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	items: Vec<crate::xml::import_sections::h_narrative::HSenderDiagnosisImport>,
) -> Result<()> {
	for item in items {
		let existing: Option<(Uuid,)> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as(
					"SELECT id FROM sender_diagnoses WHERE narrative_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(narrative_id)
				.bind(item.sequence_number),
			)
			.await
			.map_err(model::Error::from)?;
		if let Some((id,)) = existing {
			SenderDiagnosisBmc::update(
				ctx,
				mm,
				id,
				SenderDiagnosisForUpdate {
					diagnosis_meddra_version: item.diagnosis_meddra_version,
					diagnosis_meddra_code: item.diagnosis_meddra_code,
				},
			)
			.await?;
		} else {
			let id = SenderDiagnosisBmc::create(
				ctx,
				mm,
				SenderDiagnosisForCreate {
					narrative_id,
					sequence_number: item.sequence_number,
					diagnosis_meddra_version: item.diagnosis_meddra_version.clone(),
					diagnosis_meddra_code: item.diagnosis_meddra_code.clone(),
				},
			)
			.await?;
			SenderDiagnosisBmc::update(
				ctx,
				mm,
				id,
				SenderDiagnosisForUpdate {
					diagnosis_meddra_version: item.diagnosis_meddra_version,
					diagnosis_meddra_code: item.diagnosis_meddra_code,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn import_case_summaries(
	ctx: &Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	items: Vec<crate::xml::import_sections::h_narrative::HCaseSummaryImport>,
) -> Result<()> {
	for item in items {
		let existing: Option<(Uuid,)> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as(
					"SELECT id FROM case_summary_information WHERE narrative_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(narrative_id)
				.bind(item.sequence_number),
			)
			.await
			.map_err(model::Error::from)?;
		if let Some((id,)) = existing {
			CaseSummaryInformationBmc::update(
				ctx,
				mm,
				id,
				CaseSummaryInformationForUpdate {
					summary_type: item.summary_type,
					language_code: item.language_code,
					summary_text: item.summary_text,
				},
			)
			.await?;
		} else {
			let id = CaseSummaryInformationBmc::create(
				ctx,
				mm,
				CaseSummaryInformationForCreate {
					narrative_id,
					sequence_number: item.sequence_number,
					summary_type: item.summary_type.clone(),
					language_code: item.language_code.clone(),
					summary_text: item.summary_text.clone(),
				},
			)
			.await?;
			CaseSummaryInformationBmc::update(
				ctx,
				mm,
				id,
				CaseSummaryInformationForUpdate {
					summary_type: item.summary_type,
					language_code: item.language_code,
					summary_text: item.summary_text,
				},
			)
			.await?;
		}
	}
	Ok(())
}
