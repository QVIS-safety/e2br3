use crate::test_common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::ctx::Ctx;
use lib_core::model::drug::{DrugInformationBmc, DrugInformationForCreate};
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate,
};
use lib_core::model::patient::{PatientInformationBmc, PatientInformationForCreate};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::safety_report::{
	StudyInformationBmc, StudyInformationForCreate,
};
use lib_core::model::ModelManager;
use rust_decimal::Decimal;
use sqlx::types::time::{Date, OffsetDateTime, PrimitiveDateTime, Time};
use sqlx::types::Uuid;
use time::Month;

pub async fn setup_case() -> Result<(ModelManager, Ctx, Uuid)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	Ok((mm, ctx, case_id))
}

pub async fn finish(mm: &ModelManager) -> Result<()> {
	commit_test_ctx(mm).await
}

pub async fn create_patient(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	PatientInformationBmc::create(
		ctx,
		mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("PT".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await
	.map_err(Into::into)
}

pub async fn create_drug(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	DrugInformationBmc::create(
		ctx,
		mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug".to_string(),
			drug_generic_name: None,
			..Default::default()
		},
	)
	.await
	.map_err(Into::into)
}

pub async fn create_reaction(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	ReactionBmc::create(
		ctx,
		mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Reaction".to_string(),
		},
	)
	.await
	.map_err(Into::into)
}

pub async fn create_narrative(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	NarrativeInformationBmc::create(
		ctx,
		mm,
		NarrativeInformationForCreate {
			case_id,
			case_narrative: "Narrative".to_string(),
		},
	)
	.await
	.map_err(Into::into)
}

pub async fn create_study(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	StudyInformationBmc::create(
		ctx,
		mm,
		StudyInformationForCreate {
			case_id,
			study_name: Some("Study".to_string()),
			sponsor_study_number: Some("SPONSOR".to_string()),
			study_type_reaction: Some("1".to_string()),
			study_type_reaction_kr1: Some("Other".to_string()),
		},
	)
	.await
	.map_err(Into::into)
}

pub fn date(y: i32, m: Month, d: u8) -> Date {
	Date::from_calendar_date(y, m, d).unwrap()
}

pub fn datetime_utc(
	y: i32,
	m: Month,
	d: u8,
	h: u8,
	min: u8,
	s: u8,
) -> OffsetDateTime {
	PrimitiveDateTime::new(date(y, m, d), Time::from_hms(h, min, s).unwrap())
		.assume_utc()
}

pub fn dec(v: i64, scale: u32) -> Decimal {
	Decimal::new(v, scale)
}
