mod common;

use crate::common::{
	demo_ctx, demo_org_id, demo_user_id, reset_role, set_auditor_role,
	system_user_id, Result,
};
use lib_core::_dev_utils;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::presave::{
	NarrativePresaveBmc, NarrativePresaveForCreate,
	ProductPresaveActiveSubstanceBmc, ProductPresaveActiveSubstanceForCreate,
	ProductPresaveActiveSubstanceForUpdate, ProductPresaveBmc,
	ProductPresaveForCreate, ProductPresaveForUpdate, ReceiverPresaveBmc,
	ReceiverPresaveConsigneeBmc, ReceiverPresaveConsigneeForCreate,
	ReceiverPresaveConsigneeForUpdate, ReceiverPresaveForCreate,
	ReceiverPresaveForUpdate, ReceiverPresaveRouteBmc,
	ReceiverPresaveRouteForCreate, ReceiverPresaveRouteForUpdate,
	ReporterPresaveBmc, ReporterPresaveForCreate, ReporterPresaveForUpdate,
	SenderPresaveBmc, SenderPresaveForCreate, SenderPresaveForUpdate,
	SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate, SenderPresaveResponsiblePersonBmc,
	SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresaveBmc,
	StudyPresaveFdaCrossReportedIndNumberBmc,
	StudyPresaveFdaCrossReportedIndNumberForCreate,
	StudyPresaveFdaCrossReportedIndNumberForUpdate, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveProductBmc, StudyPresaveProductForCreate,
	StudyPresaveProductForUpdate, StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
};
use lib_core::model::presave_lifecycle::{PresaveKind, PresaveLifecycleService};
use lib_core::model::store::{set_org_context, set_user_context};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use serde_json::json;
use serial_test::serial;
use std::collections::HashSet;

use rust_decimal::Decimal;
use sqlx::types::Uuid;
use sqlx::Error as SqlxError;

const SECTION_PRESAVE_TABLES: &[&str] = &[
	"sender_presaves",
	"sender_presave_gateways",
	"sender_presave_responsible_persons",
	"receiver_presaves",
	"receiver_presave_consignees",
	"receiver_presave_routes",
	"product_presaves",
	"product_presave_active_substances",
	"reporter_presaves",
	"study_presaves",
	"study_presave_registration_numbers",
	"study_presave_fda_cross_reported_ind_numbers",
	"study_presave_products",
	"study_presave_reporters",
	"narrative_presaves",
];

#[path = "presave/access.rs"]
mod access;
#[path = "presave/audit.rs"]
mod audit;
#[path = "presave/children.rs"]
mod children;
#[path = "presave/crud.rs"]
mod crud;
#[path = "presave/lifecycle.rs"]
mod lifecycle;
#[path = "presave/schema.rs"]
mod schema;
#[path = "presave/validation.rs"]
mod validation;

fn is_inactive_presave_reference(err: &SqlxError) -> bool {
	match err {
		SqlxError::Database(db_err) => db_err.code().as_deref() == Some("P2001"),
		_ => false,
	}
}

fn expect_validation_error<T>(result: lib_core::model::Result<T>, expected: &str) {
	match result {
		Err(ModelError::Validation { message }) => assert!(
			message.contains(expected),
			"expected Validation error containing {expected:?}, got {message:?}"
		),
		Err(err) => {
			panic!("expected Validation error containing {expected:?}, got {err:?}")
		}
		Ok(_) => panic!("expected Validation error containing {expected:?}, got Ok"),
	}
}

fn expect_conflict_error<T>(result: lib_core::model::Result<T>, expected: &str) {
	match result {
		Err(ModelError::Conflict { message }) => assert!(
			message.contains(expected),
			"expected Conflict error containing {expected:?}, got {message:?}"
		),
		Err(err) => {
			panic!("expected Conflict error containing {expected:?}, got {err:?}")
		}
		Ok(_) => panic!("expected Conflict error containing {expected:?}, got Ok"),
	}
}

fn sponsor_ctx(org_id: Uuid, role: &str) -> Ctx {
	Ctx::new(demo_user_id(), org_id, role.to_string())
		.expect("failed to create sponsor context")
}

async fn create_acl_test_org(mm: &ModelManager, label: &str) -> Result<Uuid> {
	let org_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), "system_admin").await?;
	sqlx::query(
		"INSERT INTO organizations (
			id, name, org_type, active, created_by, created_at, updated_at
		) VALUES (
			$1, $2, 'sponsor', true, $3, NOW(), NOW()
		)",
	)
	.bind(org_id)
	.bind(format!("ACL Test Org {label} {org_id}"))
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(org_id)
}

async fn latest_audit_changed_fields(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
) -> Result<serde_json::Value> {
	set_auditor_role(mm).await?;
	let changed_fields_result = sqlx::query_scalar(
		"SELECT changed_fields
		 FROM audit_logs
		 WHERE table_name = $1
		   AND record_id = $2
		   AND action = 'UPDATE'
		 ORDER BY created_at DESC, id DESC
		 LIMIT 1",
	)
	.bind(table_name)
	.bind(record_id)
	.fetch_one(mm.dbx().db())
	.await;
	reset_role(mm).await?;

	Ok(changed_fields_result?)
}

async fn assert_audit_changed_field(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
	field_name: &str,
	expected_old: serde_json::Value,
	expected_new: serde_json::Value,
) -> Result<()> {
	let changed_fields =
		latest_audit_changed_fields(mm, table_name, record_id).await?;
	assert_eq!(
		changed_fields.get(field_name),
		Some(&json!({
			"old": expected_old,
			"new": expected_new
		})),
		"expected {table_name}.{field_name} audit diff for {record_id}, got {changed_fields}"
	);
	Ok(())
}

fn product_presave_create(
	_authority: RegulatoryAuthority,
	_name: String,
	sender_presave_id: Uuid,
) -> ProductPresaveForCreate {
	ProductPresaveForCreate {
		sender_presave_id: Some(sender_presave_id),
		receiver_presave_id: None,
		product_id: Some(format!("PRODUCT-{}", Uuid::new_v4())),
		medicinal_product: Some("Authority Product".into()),
		medicinal_product_notation: None,
		preapproval_ip_name: None,
		brand_name: None,
		original_manufacturer: None,
		product_description: None,
		mpid: None,
		mpid_version: None,
		mfds_mpid: None,
		mfds_mpid_version: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		drug_authorization_country: None,
		drug_authorization_holder: None,
		holder_applicant_name_notation: None,
	}
}

fn receiver_presave_create(organization_name: String) -> ReceiverPresaveForCreate {
	ReceiverPresaveForCreate {
		receiver_type: Some("Regulatory Authority".into()),
		organization_name: Some(organization_name),
		day_count_rule: None,
		nsae_solicited_day_count: None,
		nsae_solicited_not_applicable: None,
		nsae_non_solicited_day_count: None,
		nsae_non_solicited_not_applicable: None,
		sae_solicited_day_count: None,
		sae_solicited_not_applicable: None,
		sae_non_solicited_day_count: None,
		sae_non_solicited_not_applicable: None,
		description: None,
	}
}

fn sender_presave_create(_name: String) -> SenderPresaveForCreate {
	SenderPresaveForCreate {
		is_default: None,
		sender_type: Some("1".into()),
		organization_name: Some(format!("Sender Org {}", Uuid::new_v4())),
		organization_name_notation: None,
		street_address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		telephone: None,
		fax: None,
		email: None,
	}
}

fn reporter_presave_create(
	_authority: RegulatoryAuthority,
	_name: String,
) -> ReporterPresaveForCreate {
	ReporterPresaveForCreate {
		reporter_title: None,
		reporter_given_name: Some("Authority".into()),
		reporter_middle_name: None,
		reporter_family_name: Some("Reporter".into()),
		organization: Some(format!("Authority Reporter Org {}", Uuid::new_v4())),
		department: None,
		street: None,
		city: None,
		state: None,
		postcode: None,
		telephone: None,
		country_code: Some("KR".into()),
		qualification: Some("1".into()),
		qualification_kr1: None,
		qualification_null_flavor: None,
		primary_source_regulatory: None,
		..Default::default()
	}
}

fn study_presave_create(
	_authority: RegulatoryAuthority,
	_name: String,
) -> StudyPresaveForCreate {
	StudyPresaveForCreate {
		product_presave_id: None,
		study_name: Some("Authority Study".into()),
		study_name_null_flavor: None,
		study_name_notation: None,
		sponsor_study_number: Some("AUTH-STUDY".into()),
		sponsor_study_number_null_flavor: None,
		sponsor_study_number_kind: None,
		study_type_reaction: Some("1".into()),
		study_type_reaction_kr1: None,
		fda_ind_number_occurred: None,
		fda_pre_anda_number_occurred: None,
		edc_sync: None,
		exclude_case_key_from_sync: None,
	}
}

fn study_presave_create_for_product(
	_name: String,
	product_presave_id: Uuid,
) -> StudyPresaveForCreate {
	StudyPresaveForCreate {
		product_presave_id: Some(product_presave_id),
		study_name: Some("Relationship Study".into()),
		study_name_null_flavor: None,
		study_name_notation: None,
		sponsor_study_number: Some(format!("REL-STUDY-{}", Uuid::new_v4())),
		sponsor_study_number_null_flavor: None,
		sponsor_study_number_kind: None,
		study_type_reaction: Some("1".into()),
		study_type_reaction_kr1: None,
		fda_ind_number_occurred: None,
		fda_pre_anda_number_occurred: None,
		edc_sync: None,
		exclude_case_key_from_sync: None,
	}
}
