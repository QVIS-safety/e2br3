mod common;

use common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, init_test_mm,
	rollback_test_ctx, set_current_user, unique_suffix, Result,
};
use lib_core::ctx::{Ctx, ROLE_ADMIN, ROLE_USER, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	RelatednessAssessmentBmc, RelatednessAssessmentForCreate,
};
use lib_core::model::drug_recurrence::{
	DrugRecurrenceInformationBmc, DrugRecurrenceInformationForCreate,
};
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	NarrativeInformationBmc, NarrativeInformationForCreate, SenderDiagnosisBmc,
	SenderDiagnosisForCreate,
};
use lib_core::model::organization::{OrganizationBmc, OrganizationForCreate};
use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryForCreate,
	ParentPastDrugHistoryBmc, ParentPastDrugHistoryForCreate,
};
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, ParentInformationBmc,
	ParentInformationForCreate, PastDrugHistoryBmc, PastDrugHistoryForCreate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientInformationBmc, PatientInformationForCreate, ReportedCauseOfDeathBmc,
	ReportedCauseOfDeathForCreate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::safety_report::{
	StudyInformationBmc, StudyInformationForCreate, StudyRegistrationNumberBmc,
	StudyRegistrationNumberForCreate,
};
use lib_core::model::store::set_org_context_dbx;
use lib_core::model::user::{UserBmc, UserForCreate};
use lib_core::model::Error as ModelError;
use serial_test::serial;
use sqlx::{query_as, types::Uuid};

async fn enable_rls(mm: &lib_core::model::ModelManager) -> Result<()> {
	mm.dbx()
		.execute(sqlx::query("SET ROLE e2br3_app_role"))
		.await?;
	mm.dbx()
		.execute(sqlx::query("SET row_security = on"))
		.await?;
	Ok(())
}

fn system_user_id() -> Uuid {
	Uuid::parse_str(SYSTEM_USER_ID).expect("system user id")
}

fn system_org_id() -> Uuid {
	Uuid::parse_str(SYSTEM_ORG_ID).expect("system org id")
}

async fn create_org(mm: &lib_core::model::ModelManager, ctx: &Ctx) -> Result<Uuid> {
	let suffix = unique_suffix();
	let org_c = OrganizationForCreate {
		name: format!("RLS Org {suffix}"),
		org_type: Some("internal".to_string()),
		address: Some("123 RLS St".to_string()),
		contact_email: Some(format!("rls-org-{suffix}@example.com")),
	};
	Ok(OrganizationBmc::create(ctx, mm, org_c).await?)
}

async fn create_user(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	org_id: Uuid,
) -> Result<Uuid> {
	let suffix = unique_suffix();
	let user_c = UserForCreate {
		organization_id: org_id,
		email: format!("rls-user-{suffix}@example.com"),
		username: format!("rls_user_{suffix}"),
		pwd_clear: "pwd123".to_string(),
		role: Some(ROLE_USER.to_string()),
		first_name: None,
		last_name: None,
	};
	Ok(UserBmc::create(ctx, mm, user_c).await?)
}

async fn create_patient(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Uuid> {
	let data = PatientInformationForCreate {
		case_id,
		patient_initials: Some("AB".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	Ok(PatientInformationBmc::create(ctx, mm, data).await?)
}

async fn create_drug(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Uuid> {
	let data = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
	};
	Ok(DrugInformationBmc::create(ctx, mm, data).await?)
}

async fn create_reaction(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Uuid> {
	let data = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
	};
	Ok(ReactionBmc::create(ctx, mm, data).await?)
}

async fn create_narrative(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Uuid> {
	let data = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Test narrative".to_string(),
	};
	Ok(NarrativeInformationBmc::create(ctx, mm, data).await?)
}

async fn create_study(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
) -> Result<Uuid> {
	let data = StudyInformationForCreate {
		case_id,
		study_name: Some("Study".to_string()),
		sponsor_study_number: Some("SN-1".to_string()),
	};
	Ok(StudyInformationBmc::create(ctx, mm, data).await?)
}

async fn create_patient_death(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	patient_id: Uuid,
) -> Result<Uuid> {
	let data = PatientDeathInformationForCreate {
		patient_id,
		date_of_death: None,
		autopsy_performed: None,
	};
	Ok(PatientDeathInformationBmc::create(ctx, mm, data).await?)
}

async fn create_parent_info(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	patient_id: Uuid,
) -> Result<Uuid> {
	let data = ParentInformationForCreate {
		patient_id,
		sex: None,
		medical_history_text: None,
	};
	Ok(ParentInformationBmc::create(ctx, mm, data).await?)
}

async fn create_drug_reaction_assessment(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	drug_id: Uuid,
	reaction_id: Uuid,
) -> Result<Uuid> {
	let data = DrugReactionAssessmentForCreate {
		drug_id,
		reaction_id,
	};
	Ok(DrugReactionAssessmentBmc::create(ctx, mm, data).await?)
}

fn assert_denied<T>(res: core::result::Result<T, ModelError>) {
	assert!(matches!(res, Err(ModelError::EntityUuidNotFound { .. })));
}

#[serial]
#[tokio::test]
async fn test_rls_case_related_tables_org_isolation() -> Result<()> {
	let mm = init_test_mm().await;
	let admin_ctx =
		Ctx::new(system_user_id(), system_org_id(), ROLE_ADMIN.to_string())?;

	let org1_id = create_org(&mm, &admin_ctx).await?;
	let user1_id = create_user(&mm, &admin_ctx, org1_id).await?;

	let org2_id = create_org(&mm, &admin_ctx).await?;
	let user2_id = create_user(&mm, &admin_ctx, org2_id).await?;

	set_current_user(&mm, user1_id).await?;
	let ctx = Ctx::new(user1_id, org1_id, ROLE_USER.to_string())?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_org1 = create_case_fixture(&mm, org1_id, user1_id).await?;
	set_current_user(&mm, user2_id).await?;
	let _ctx = Ctx::new(user2_id, org2_id, ROLE_USER.to_string())?;
	let case_org2 = create_case_fixture(&mm, org2_id, user2_id).await?;

	let patient1 = create_patient(&mm, &admin_ctx, case_org1).await?;
	let patient2 = create_patient(&mm, &admin_ctx, case_org2).await?;
	let drug1 = create_drug(&mm, &admin_ctx, case_org1).await?;
	let drug2 = create_drug(&mm, &admin_ctx, case_org2).await?;
	let reaction1 = create_reaction(&mm, &admin_ctx, case_org1).await?;
	let reaction2 = create_reaction(&mm, &admin_ctx, case_org2).await?;
	let narrative1 = create_narrative(&mm, &admin_ctx, case_org1).await?;
	let narrative2 = create_narrative(&mm, &admin_ctx, case_org2).await?;
	let study1 = create_study(&mm, &admin_ctx, case_org1).await?;
	let study2 = create_study(&mm, &admin_ctx, case_org2).await?;

	let death1 = create_patient_death(&mm, &admin_ctx, patient1).await?;
	let death2 = create_patient_death(&mm, &admin_ctx, patient2).await?;
	let parent1 = create_parent_info(&mm, &admin_ctx, patient1).await?;
	let parent2 = create_parent_info(&mm, &admin_ctx, patient2).await?;
	let dra1 =
		create_drug_reaction_assessment(&mm, &admin_ctx, drug1, reaction1).await?;
	let dra2 =
		create_drug_reaction_assessment(&mm, &admin_ctx, drug2, reaction2).await?;

	let med_hist1 = MedicalHistoryEpisodeBmc::create(
		&admin_ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id: patient1,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	let med_hist2 = MedicalHistoryEpisodeBmc::create(
		&admin_ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id: patient2,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;

	let past_drug1 = PastDrugHistoryBmc::create(
		&admin_ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id: patient1,
			sequence_number: 1,
			drug_name: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			end_date: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;
	let past_drug2 = PastDrugHistoryBmc::create(
		&admin_ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id: patient2,
			sequence_number: 1,
			drug_name: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			end_date: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;

	let reported1 = ReportedCauseOfDeathBmc::create(
		&admin_ctx,
		&mm,
		ReportedCauseOfDeathForCreate {
			death_info_id: death1,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	let reported2 = ReportedCauseOfDeathBmc::create(
		&admin_ctx,
		&mm,
		ReportedCauseOfDeathForCreate {
			death_info_id: death2,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;

	let autopsy1 = AutopsyCauseOfDeathBmc::create(
		&admin_ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id: death1,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	let autopsy2 = AutopsyCauseOfDeathBmc::create(
		&admin_ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id: death2,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;

	let parent_med1 = ParentMedicalHistoryBmc::create(
		&admin_ctx,
		&mm,
		ParentMedicalHistoryForCreate {
			parent_id: parent1,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	let parent_med2 = ParentMedicalHistoryBmc::create(
		&admin_ctx,
		&mm,
		ParentMedicalHistoryForCreate {
			parent_id: parent2,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;

	let parent_past1 = ParentPastDrugHistoryBmc::create(
		&admin_ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id: parent1,
			sequence_number: 1,
			drug_name: None,
		},
	)
	.await?;
	let parent_past2 = ParentPastDrugHistoryBmc::create(
		&admin_ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id: parent2,
			sequence_number: 1,
			drug_name: None,
		},
	)
	.await?;

	let active_sub1 = DrugActiveSubstanceBmc::create(
		&admin_ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id: drug1,
			sequence_number: 1,
			substance_name: None,
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;
	let active_sub2 = DrugActiveSubstanceBmc::create(
		&admin_ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id: drug2,
			sequence_number: 1,
			substance_name: None,
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let dosage1 = DosageInformationBmc::create(
		&admin_ctx,
		&mm,
		DosageInformationForCreate {
			drug_id: drug1,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			batch_lot_number: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;
	let dosage2 = DosageInformationBmc::create(
		&admin_ctx,
		&mm,
		DosageInformationForCreate {
			drug_id: drug2,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			batch_lot_number: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;

	let indication1 = DrugIndicationBmc::create(
		&admin_ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id: drug1,
			sequence_number: 1,
			indication_text: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
		},
	)
	.await?;
	let indication2 = DrugIndicationBmc::create(
		&admin_ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id: drug2,
			sequence_number: 1,
			indication_text: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
		},
	)
	.await?;

	let sender_diag1 = SenderDiagnosisBmc::create(
		&admin_ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id: narrative1,
			sequence_number: 1,
			diagnosis_meddra_code: None,
		},
	)
	.await?;
	let sender_diag2 = SenderDiagnosisBmc::create(
		&admin_ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id: narrative2,
			sequence_number: 1,
			diagnosis_meddra_code: None,
		},
	)
	.await?;

	let case_summary1 = CaseSummaryInformationBmc::create(
		&admin_ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id: narrative1,
			sequence_number: 1,
			summary_text: None,
		},
	)
	.await?;
	let case_summary2 = CaseSummaryInformationBmc::create(
		&admin_ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id: narrative2,
			sequence_number: 1,
			summary_text: None,
		},
	)
	.await?;

	let recurrence1 = DrugRecurrenceInformationBmc::create(
		&admin_ctx,
		&mm,
		DrugRecurrenceInformationForCreate {
			drug_id: drug1,
			sequence_number: 1,
		},
	)
	.await?;
	let recurrence2 = DrugRecurrenceInformationBmc::create(
		&admin_ctx,
		&mm,
		DrugRecurrenceInformationForCreate {
			drug_id: drug2,
			sequence_number: 1,
		},
	)
	.await?;

	let related1 = RelatednessAssessmentBmc::create(
		&admin_ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: dra1,
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: None,
			result_of_assessment: None,
		},
	)
	.await?;
	let related2 = RelatednessAssessmentBmc::create(
		&admin_ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: dra2,
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: None,
			result_of_assessment: None,
		},
	)
	.await?;

	let study_reg1 = StudyRegistrationNumberBmc::create(
		&admin_ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study1,
			registration_number: "REG-1".to_string(),
			country_code: None,
			sequence_number: 1,
		},
	)
	.await?;
	let study_reg2 = StudyRegistrationNumberBmc::create(
		&admin_ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study2,
			registration_number: "REG-2".to_string(),
			country_code: None,
			sequence_number: 1,
		},
	)
	.await?;

	commit_test_ctx(&mm).await?;
	let _dbx = mm.dbx();
	let user_ctx = Ctx::new(user1_id, org1_id, ROLE_USER.to_string())?;
	begin_test_ctx(&mm, &user_ctx).await?;
	enable_rls(&mm).await?;
	assert!(CaseBmc::get(&user_ctx, &mm, case_org1).await.is_ok());
	assert_denied(CaseBmc::get(&user_ctx, &mm, case_org2).await);

	assert!(MedicalHistoryEpisodeBmc::get(&user_ctx, &mm, med_hist1)
		.await
		.is_ok());
	assert_denied(MedicalHistoryEpisodeBmc::get(&user_ctx, &mm, med_hist2).await);

	assert!(PastDrugHistoryBmc::get(&user_ctx, &mm, past_drug1)
		.await
		.is_ok());
	assert_denied(PastDrugHistoryBmc::get(&user_ctx, &mm, past_drug2).await);

	assert!(PatientDeathInformationBmc::get(&user_ctx, &mm, death1)
		.await
		.is_ok());
	assert_denied(PatientDeathInformationBmc::get(&user_ctx, &mm, death2).await);

	assert!(ReportedCauseOfDeathBmc::get(&user_ctx, &mm, reported1)
		.await
		.is_ok());
	assert_denied(ReportedCauseOfDeathBmc::get(&user_ctx, &mm, reported2).await);

	assert!(AutopsyCauseOfDeathBmc::get(&user_ctx, &mm, autopsy1)
		.await
		.is_ok());
	assert_denied(AutopsyCauseOfDeathBmc::get(&user_ctx, &mm, autopsy2).await);

	assert!(ParentInformationBmc::get(&user_ctx, &mm, parent1)
		.await
		.is_ok());
	assert_denied(ParentInformationBmc::get(&user_ctx, &mm, parent2).await);

	assert!(ParentMedicalHistoryBmc::get(&user_ctx, &mm, parent_med1)
		.await
		.is_ok());
	assert_denied(ParentMedicalHistoryBmc::get(&user_ctx, &mm, parent_med2).await);

	assert!(ParentPastDrugHistoryBmc::get(&user_ctx, &mm, parent_past1)
		.await
		.is_ok());
	assert_denied(ParentPastDrugHistoryBmc::get(&user_ctx, &mm, parent_past2).await);

	assert!(DrugActiveSubstanceBmc::get(&user_ctx, &mm, active_sub1)
		.await
		.is_ok());
	assert_denied(DrugActiveSubstanceBmc::get(&user_ctx, &mm, active_sub2).await);

	assert!(DosageInformationBmc::get(&user_ctx, &mm, dosage1)
		.await
		.is_ok());
	assert_denied(DosageInformationBmc::get(&user_ctx, &mm, dosage2).await);

	assert!(DrugIndicationBmc::get(&user_ctx, &mm, indication1)
		.await
		.is_ok());
	assert_denied(DrugIndicationBmc::get(&user_ctx, &mm, indication2).await);

	assert!(SenderDiagnosisBmc::get(&user_ctx, &mm, sender_diag1)
		.await
		.is_ok());
	assert_denied(SenderDiagnosisBmc::get(&user_ctx, &mm, sender_diag2).await);

	assert!(
		CaseSummaryInformationBmc::get(&user_ctx, &mm, case_summary1)
			.await
			.is_ok()
	);
	assert_denied(
		CaseSummaryInformationBmc::get(&user_ctx, &mm, case_summary2).await,
	);

	assert!(
		DrugRecurrenceInformationBmc::get(&user_ctx, &mm, recurrence1)
			.await
			.is_ok()
	);
	assert_denied(
		DrugRecurrenceInformationBmc::get(&user_ctx, &mm, recurrence2).await,
	);

	assert!(RelatednessAssessmentBmc::get(&user_ctx, &mm, related1)
		.await
		.is_ok());
	assert_denied(RelatednessAssessmentBmc::get(&user_ctx, &mm, related2).await);

	assert!(StudyRegistrationNumberBmc::get(&user_ctx, &mm, study_reg1)
		.await
		.is_ok());
	assert_denied(StudyRegistrationNumberBmc::get(&user_ctx, &mm, study_reg2).await);

	rollback_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rls_terminology_admin_only() -> Result<()> {
	let mm = init_test_mm().await;
	let admin_ctx =
		Ctx::new(system_user_id(), system_org_id(), ROLE_ADMIN.to_string())?;
	let org1_id = create_org(&mm, &admin_ctx).await?;
	let _user1_id = create_user(&mm, &admin_ctx, org1_id).await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	enable_rls(&mm).await?;

	set_org_context_dbx(dbx, org1_id, ROLE_USER).await?;
	let (count_user,): (i64,) = dbx
		.fetch_one(query_as::<_, (i64,)>("SELECT COUNT(*) FROM meddra_terms"))
		.await?;
	assert_eq!(count_user, 0);

	set_org_context_dbx(dbx, org1_id, ROLE_ADMIN).await?;
	let _: (i64,) = dbx
		.fetch_one(query_as::<_, (i64,)>("SELECT COUNT(*) FROM meddra_terms"))
		.await?;

	dbx.rollback_txn().await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
