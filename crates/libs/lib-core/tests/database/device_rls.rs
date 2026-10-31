use super::{init_test_mm, Result};
use lib_core::ctx::{
	Ctx, ROLE_SYSTEM_ADMIN, ROLE_USER, SYSTEM_ORG_ID, SYSTEM_USER_ID,
};
use lib_core::model::drug::{
	DrugDeviceCharacteristicBmc, DrugDeviceCharacteristicFilter, FdaDeviceCodeBmc,
	FdaDeviceCodeFilter, FdaDeviceInformationBmc, FdaDeviceInformationFilter,
};
use lib_core::model::store::{set_org_context, set_user_context};
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn uuid_filter(id: Uuid) -> Option<OpValsValue> {
	Some(OpValsValue::from(vec![OpValValue::Eq(json!(
		id.to_string()
	))]))
}

#[serial]
#[tokio::test(flavor = "multi_thread")]
async fn device_bmcs_enforce_tenant_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let tenant_org_id = Uuid::new_v4();
	let other_org_id = Uuid::new_v4();
	let tenant_user_id = Uuid::new_v4();
	let tenant_case_id = Uuid::new_v4();
	let other_case_id = Uuid::new_v4();
	let tenant_drug_id = Uuid::new_v4();
	let other_drug_id = Uuid::new_v4();
	let tenant_device_id = Uuid::new_v4();
	let other_device_id = Uuid::new_v4();

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;
	for (id, name) in [
		(tenant_org_id, "Device RLS tenant"),
		(other_org_id, "Device RLS other tenant"),
	] {
		sqlx::query(
			"INSERT INTO organizations (id, name, org_type, created_by, updated_by)
			 VALUES ($1, $2, 'cro', $3, $3)",
		)
		.bind(id)
		.bind(name)
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
	}
	sqlx::query(
		"INSERT INTO users (
			id, organization_id, email, username, token_salt, role, active,
			created_by, updated_by
		 ) VALUES ($1, $2, $3, $4, $5, $6, true, $7, $7)",
	)
	.bind(tenant_user_id)
	.bind(tenant_org_id)
	.bind(format!("device-rls-{tenant_user_id}@example.com"))
	.bind(format!("device_rls_{tenant_user_id}"))
	.bind(Uuid::new_v4())
	.bind(ROLE_USER)
	.bind(system_user_id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (
			user_id, organization_id, created_by, updated_by
		 ) VALUES ($1, $2, $3, $3)",
	)
	.bind(tenant_user_id)
	.bind(tenant_org_id)
	.bind(system_user_id)
	.execute(&mut *tx)
	.await?;
	for (id, organization_id) in [
		(tenant_case_id, tenant_org_id),
		(other_case_id, other_org_id),
	] {
		sqlx::query(
			"INSERT INTO cases (id, organization_id, created_by, updated_by)
			 VALUES ($1, $2, $3, $3)",
		)
		.bind(id)
		.bind(organization_id)
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
	}
	for (id, case_id, product) in [
		(tenant_drug_id, tenant_case_id, "Tenant product"),
		(other_drug_id, other_case_id, "Other tenant product"),
	] {
		sqlx::query(
			"INSERT INTO drug_information (
				id, case_id, sequence_number, drug_characterization,
				medicinal_product, created_by
			 ) VALUES ($1, $2, 1, '1', $3, $4)",
		)
		.bind(id)
		.bind(case_id)
		.bind(product)
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
	}
	for (id, drug_id, brand) in [
		(tenant_device_id, tenant_drug_id, "TENANT-DEVICE"),
		(other_device_id, other_drug_id, "OTHER-DEVICE"),
	] {
		sqlx::query(
			"INSERT INTO fda_device_information (
				id, drug_id, sequence_number, device_brand_name, created_by
			 ) VALUES ($1, $2, 1, $3, $4)",
		)
		.bind(id)
		.bind(drug_id)
		.bind(brand)
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
		sqlx::query(
			"INSERT INTO fda_device_codes (
				id, device_id, element, sequence_number, value_code, created_by
			 ) VALUES ($1, $2, 'device_problem', 1, $3, $4)",
		)
		.bind(Uuid::new_v4())
		.bind(id)
		.bind(if drug_id == tenant_drug_id {
			"7654321"
		} else {
			"1234567"
		})
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
		sqlx::query(
			"INSERT INTO drug_device_characteristics (
				id, drug_id, sequence_number, code, value_type, value_value,
				created_by
			 ) VALUES ($1, $2, 1, $3, 'ST', 'device characteristic', $4)",
		)
		.bind(Uuid::new_v4())
		.bind(drug_id)
		.bind(if drug_id == tenant_drug_id {
			"TENANT.CHAR"
		} else {
			"OTHER.CHAR"
		})
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
	}
	tx.commit().await?;

	let tenant_ctx = Ctx::new(tenant_user_id, tenant_org_id, ROLE_USER.to_string())?;
	let options = || Some(ListOptions::default());
	assert_eq!(
		DrugDeviceCharacteristicBmc::list(
			&tenant_ctx,
			&mm,
			Some(vec![DrugDeviceCharacteristicFilter {
				drug_id: uuid_filter(tenant_drug_id),
				..Default::default()
			}]),
			options(),
		)
		.await?
		.len(),
		1
	);
	assert!(DrugDeviceCharacteristicBmc::list(
		&tenant_ctx,
		&mm,
		Some(vec![DrugDeviceCharacteristicFilter {
			drug_id: uuid_filter(other_drug_id),
			..Default::default()
		}]),
		options(),
	)
	.await?
	.is_empty());
	assert_eq!(
		FdaDeviceInformationBmc::list(
			&tenant_ctx,
			&mm,
			Some(vec![FdaDeviceInformationFilter {
				drug_id: uuid_filter(tenant_drug_id),
				..Default::default()
			}]),
			options(),
		)
		.await?
		.len(),
		1
	);
	assert!(FdaDeviceInformationBmc::list(
		&tenant_ctx,
		&mm,
		Some(vec![FdaDeviceInformationFilter {
			drug_id: uuid_filter(other_drug_id),
			..Default::default()
		}]),
		options(),
	)
	.await?
	.is_empty());
	assert_eq!(
		FdaDeviceCodeBmc::list(
			&tenant_ctx,
			&mm,
			Some(vec![FdaDeviceCodeFilter {
				device_id: uuid_filter(tenant_device_id),
				..Default::default()
			}]),
			options(),
		)
		.await?
		.len(),
		1
	);
	assert!(FdaDeviceCodeBmc::list(
		&tenant_ctx,
		&mm,
		Some(vec![FdaDeviceCodeFilter {
			device_id: uuid_filter(other_device_id),
			..Default::default()
		}]),
		options(),
	)
	.await?
	.is_empty());

	Ok(())
}
