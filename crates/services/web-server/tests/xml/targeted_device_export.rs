use crate::common::{
	init_test_mm, seed_two_orgs_manager_cases, system_org_id, system_user_id,
	Result, TEST_CUSTOM_MANAGER_ROLE,
};
use lib_core::ctx::Ctx;
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate,
};
use lib_core::model::patient::{PatientInformationBmc, PatientInformationForCreate};
use serial_test::serial;

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn targeted_device_export_ignores_same_and_cross_org_volume() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_manager_cases(&mm).await?;
	let background_case_id = uuid::Uuid::new_v4();
	let background_drug_id = uuid::Uuid::new_v4();
	let target_drug_id = uuid::Uuid::new_v4();
	let other_org_drug_id = uuid::Uuid::new_v4();
	let target_device_id =
		uuid::Uuid::parse_str("ffffffff-ffff-4fff-8fff-ffffffffffff")?;
	let other_org_device_id =
		uuid::Uuid::parse_str("eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee")?;

	let mut tx = mm.dbx().db().begin().await?;
	lib_core::model::store::set_user_context(&mut tx, system_user_id()).await?;
	lib_core::model::store::set_org_context(
		&mut tx,
		system_org_id(),
		lib_core::ctx::ROLE_SYSTEM_ADMIN,
	)
	.await?;
	sqlx::query(
		"INSERT INTO cases (id, organization_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $3)",
	)
	.bind(background_case_id)
	.bind(seed.org1_id)
	.bind(system_user_id())
	.execute(&mut *tx)
	.await?;
	for (id, case_id, product) in [
		(background_drug_id, background_case_id, "Background product"),
		(target_drug_id, seed.case_org1, "Target product"),
		(other_org_drug_id, seed.case_org2, "Other org product"),
	] {
		sqlx::query(
			"INSERT INTO drug_information
			 (id, case_id, sequence_number, drug_characterization, medicinal_product, created_by)
			 VALUES ($1, $2, 1, '1', $3, $4)",
		)
		.bind(id)
		.bind(case_id)
		.bind(product)
		.bind(system_user_id())
		.execute(&mut *tx)
		.await?;
	}
	sqlx::query(
		"INSERT INTO fda_device_information
		 (id, drug_id, sequence_number, device_brand_name, common_device_name,
		  device_product_code, created_by)
		 SELECT ('00000000-0000-4000-8000-' || lpad(n::text, 12, '0'))::uuid,
		        $1, n, 'BACKGROUND-DEVICE', 'Background device', 'BKG', $2
		 FROM generate_series(1, 1000) AS n",
	)
	.bind(background_drug_id)
	.bind(system_user_id())
	.execute(&mut *tx)
	.await?;
	for (id, drug_id, brand) in [
		(target_device_id, target_drug_id, "TARGET-DEVICE-BRAND"),
		(other_org_device_id, other_org_drug_id, "OTHER-ORG-DEVICE"),
	] {
		sqlx::query(
			"INSERT INTO fda_device_information
			 (id, drug_id, sequence_number, device_brand_name, common_device_name,
			  device_product_code, created_by)
			 VALUES ($1, $2, 1, $3, 'Regression device', 'REG', $4)",
		)
		.bind(id)
		.bind(drug_id)
		.bind(brand)
		.bind(system_user_id())
		.execute(&mut *tx)
		.await?;
	}
	for (id, device_id, value_code) in [
		(uuid::Uuid::new_v4(), target_device_id, "7654321"),
		(uuid::Uuid::new_v4(), other_org_device_id, "1234567"),
	] {
		sqlx::query(
			"INSERT INTO fda_device_codes
			 (id, device_id, element, sequence_number, value_code, created_by)
			 VALUES ($1, $2, 'device_problem', 1, $3, $4)",
		)
		.bind(id)
		.bind(device_id)
		.bind(value_code)
		.bind(system_user_id())
		.execute(&mut *tx)
		.await?;
	}
	for (drug_id, code) in [
		(target_drug_id, "TARGET.CHAR"),
		(other_org_drug_id, "OTHER.ORG.CHAR"),
	] {
		sqlx::query(
			"INSERT INTO drug_device_characteristics
			 (drug_id, sequence_number, code, value_type, value_value, created_by)
			 VALUES ($1, 1, $2, 'ST', 'device characteristic', $3)",
		)
		.bind(drug_id)
		.bind(code)
		.bind(system_user_id())
		.execute(&mut *tx)
		.await?;
	}
	sqlx::query(
		"UPDATE safety_report_identification
		 SET transmission_date = '20260821000000'
		 WHERE case_id = $1",
	)
	.bind(seed.case_org1)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let ctx = Ctx::new(
		seed.manager.id,
		seed.org1_id,
		TEST_CUSTOM_MANAGER_ROLE.to_string(),
	)?;
	PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id: seed.case_org1,
			patient_initials: Some("RLS".to_string()),
			patient_initials_null_flavor: None,
			birth_date: None,
			birth_date_null_flavor: None,
			age_at_time_of_onset: None,
			age_unit: None,
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			height_cm: None,
			sex: None,
			sex_null_flavor: Some("UNK".to_string()),
			race_codes: Vec::new(),
			race_code_null_flavor: None,
			ethnicity_code: None,
			ethnicity_code_null_flavor: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: None,
			medical_history_text_null_flavor: Some("UNK".to_string()),
			concomitant_therapy: None,
		},
	)
	.await?;
	NarrativeInformationBmc::create(
		&ctx,
		&mm,
		NarrativeInformationForCreate {
			case_id: seed.case_org1,
			source_narrative_presave_id: None,
			case_narrative: "Device export RLS regression".to_string(),
			reporter_comments: None,
			sender_comments: None,
			additional_information: None,
		},
	)
	.await?;
	let exported = tokio::task::block_in_place(|| {
		let header = xml::OutboundMessageHeader {
			batch_number: "TEST-BATCH".to_string(),
			batch_sender_identifier: "TEST-SENDER".to_string(),
			batch_receiver_identifier: "ZZFDA".to_string(),
			batch_transmission_date: time::OffsetDateTime::now_utc(),
			message_sender_identifier: "TEST-SENDER".to_string(),
			message_receiver_identifier: "CDER".to_string(),
		};
		tokio::runtime::Handle::current().block_on(
			xml::export::serialize_case_xml_for_authority(
				&ctx,
				&mm,
				seed.case_org1,
				lib_core::regulatory::RegulatoryAuthority::Fda,
				&header,
			),
		)
	})?;
	assert!(exported.contains("TARGET-DEVICE-BRAND"));
	assert!(exported.contains("code=\"7654321\""));
	assert!(!exported.contains("BACKGROUND-DEVICE"));
	assert!(!exported.contains("OTHER-ORG-DEVICE"));

	Ok(())
}
