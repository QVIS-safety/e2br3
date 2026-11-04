use crate::error::Error;
use crate::import_sections::c_safety_report::{
	import_section_c, parse_c_safety_report,
};
use crate::import_sections::d_patient::import_section_d;
use crate::import_sections::e_reaction::import_section_e;
use crate::import_sections::f_test_result::import_section_f;
use crate::import_sections::g_drug::import_section_g;
use crate::import_sections::h_narrative::import_section_h;
use crate::import_sections::shared;
use crate::types::XmlImportResult;
use crate::{parse_e2b_xml, Result};
use lib_core::ctx::Ctx;
use lib_core::model::audit::{CaseVersionBmc, CaseVersionForCreate};
use lib_core::model::case::{CaseBmc, CaseForCreate, CaseForUpdate};
use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::{self, ModelManager};
use serde_json::json;
use sqlx::types::time::Date;
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct CImportSettings {
	pub update_date_of_creation: bool,
	pub update_most_recent_info_date: bool,
	pub update_report_first_received_date: bool,
	pub apply_sender_info_to_imported_cases: bool,
	pub selected_sender_presave_id: Option<sqlx::types::Uuid>,
	pub import_date: Option<Date>,
}

#[derive(Debug, Clone)]
pub struct XmlImportRequest {
	pub xml: Vec<u8>,
	pub c_settings: CImportSettings,
	pub product_presave_id: sqlx::types::Uuid,
	pub product_id: String,
}

pub fn extract_safety_report_id_from_xml(xml: &[u8]) -> Result<String> {
	shared::extract_safety_report_id(xml)
}

fn read_import_message_header(
	header: &shared::MessageHeaderExtract,
) -> Result<(String, String, String, String)> {
	let required = |value: &Option<String>, field: &str| {
		value
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_owned)
			.ok_or_else(|| Error::InvalidImportRequest {
				message: format!("message header {field} missing"),
			})
	};
	let message_number = required(&header.message_number, "number")?;
	let message_sender = required(&header.message_sender, "sender")?;
	let message_receiver = required(&header.message_receiver, "receiver")?;
	let message_date = header
		.message_date
		.clone()
		.and_then(shared::normalize_message_date)
		.ok_or_else(|| Error::InvalidImportRequest {
			message: "message header date missing or invalid".to_string(),
		})?;

	Ok((
		message_number,
		message_sender,
		message_receiver,
		message_date,
	))
}

pub async fn import_e2b_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	let mm = mm.new_with_txn()?;
	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	let result = import_e2b_xml_in_txn(ctx, &mm, req).await;
	match result {
		Ok(result) => {
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
			Ok(result)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

/// Imports one XML document using the caller's transaction.
///
/// The supplied `ModelManager` must own an open transaction. This function
/// neither commits nor rolls it back; the caller must finish the transaction.
pub async fn import_e2b_xml_in_txn(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	let product_id = req.product_id.trim();
	if product_id.is_empty() {
		return Err(Error::InvalidImportRequest {
			message: "Product ID is required for XML import".to_string(),
		});
	}
	let product_id = product_id.to_string();
	let product_presave_id = req.product_presave_id;
	let parsed = parse_e2b_xml(&req.xml)?;
	let safety_report_id = shared::extract_safety_report_id(&req.xml)?;
	let transmission_date =
		parse_c_safety_report(&req.xml)?.and_then(|report| report.transmission_date);
	let header_extract = shared::extract_message_header(&req.xml)?;
	let (message_number, message_sender, message_receiver, message_date) =
		read_import_message_header(&header_extract)?;
	// Serialize imports for the same organization/report ID. The lock is held
	// by the outer import transaction, so the recheck below closes the race
	// between the REST decision query and case creation.
	mm.dbx()
		.execute(
			sqlx::query(
				"SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))",
			)
			.bind(format!(
				"e2b-xml-import:{}:{}",
				ctx.organization_id(),
				&safety_report_id
			)),
		)
		.await
		.map_err(model::Error::from)?;
	let (duplicate_case, next_version) = {
		let dbx = mm.dbx();
		dbx.begin_txn().await.map_err(model::Error::from)?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			let _ = dbx.rollback_txn().await;
			return Err(Error::Model(err));
		}
		let duplicate_case = dbx
			.fetch_optional(
				sqlx::query_as::<_, (Uuid, String, i32)>(
					r#"
					SELECT c.id, s.safety_report_id, s.version
					  FROM safety_report_identification s
					  JOIN cases c ON c.id = s.case_id
					 WHERE s.safety_report_id = $1
					   AND s.transmission_date = $2
					   AND c.organization_id = $3
					 ORDER BY s.version DESC
					 LIMIT 1
					"#,
				)
				.bind(&safety_report_id)
				.bind(&transmission_date)
				.bind(ctx.organization_id()),
			)
			.await
			.map_err(model::Error::from)?;
		let sql = r#"
			SELECT MAX(s.version)
			  FROM safety_report_identification s
			  JOIN cases c ON c.id = s.case_id
			 WHERE s.safety_report_id = $1
			   AND c.organization_id = $2
		"#;
		let max_version: (Option<i32>,) = dbx
			.fetch_one(
				sqlx::query_as(sql)
					.bind(&safety_report_id)
					.bind(ctx.organization_id()),
			)
			.await
			.map_err(model::Error::from)?;
		dbx.commit_txn().await.map_err(model::Error::from)?;
		(duplicate_case, max_version.0.unwrap_or(0) + 1)
	};
	if let Some((case_id, case_number, case_version)) = duplicate_case {
		return Ok(XmlImportResult {
			skipped: true,
			case_id: Some(case_id.to_string()),
			case_number: Some(case_number),
			case_version: Some(i64::from(case_version)),
			xml_key: None,
			parsed_json_id: None,
		});
	}

	let case_id = CaseBmc::create(
		ctx,
		mm,
		CaseForCreate {
			organization_id: ctx.organization_id(),
			dg_prd_key: Some(product_id.clone()),
			status: Some("draft".to_string()),
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
		},
	)
	.await?;

	let message_number =
		shared::make_import_message_number(&message_number, case_id);
	let has_header = MessageHeaderBmc::get_by_case(ctx, mm, case_id)
		.await
		.is_ok();
	if !has_header {
		MessageHeaderBmc::create(
			ctx,
			mm,
			MessageHeaderForCreate {
				case_id,
				batch_sender_identifier: None,
				batch_receiver_identifier: None,
				batch_transmission_date: None,
				message_number,
				message_sender_identifier: message_sender,
				message_receiver_identifier: message_receiver,
				message_date,
			},
		)
		.await?;
	}
	MessageHeaderBmc::update_by_case(
		ctx,
		mm,
		case_id,
		MessageHeaderForUpdate {
			batch_number: header_extract.batch_number.clone(),
			batch_sender_identifier: header_extract.batch_sender.clone(),
			batch_receiver_identifier: header_extract.batch_receiver.clone(),
			batch_transmission_date: None,
			message_number: None,
			message_sender_identifier: None,
			message_receiver_identifier: None,
			message_date: None,
		},
	)
	.await?;

	import_section_c(
		ctx,
		mm,
		&req.xml,
		case_id,
		&safety_report_id,
		next_version,
		Some(&header_extract),
		&req.c_settings,
	)
	.await?;
	import_section_d(ctx, mm, &req.xml, case_id).await?;
	import_section_h(ctx, mm, &req.xml, case_id).await?;

	let snapshot = json!({
		"parsed": parsed.json,
	});

	let reaction_map = import_section_e(ctx, mm, &req.xml, case_id).await?;
	import_section_f(ctx, mm, &req.xml, case_id).await?;
	import_section_g(
		ctx,
		mm,
		&req.xml,
		case_id,
		&reaction_map,
		product_presave_id,
	)
	.await?;

	let version_id = match CaseVersionBmc::create(
		ctx,
		mm,
		CaseVersionForCreate {
			case_id,
			version: next_version,
			snapshot,
			change_reason: Some("XML import".to_string()),
		},
	)
	.await
	{
		Ok(id) => id,
		Err(err) => return Err(err.into()),
	};

	CaseBmc::update(
		ctx,
		mm,
		case_id,
		CaseForUpdate {
			raw_xml: Some(req.xml.to_vec()),
			dg_prd_key: Some(product_id),
			status: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			submitted_by: None,
			submitted_at: None,
			dirty_c: Some(false),
			dirty_d: Some(false),
			dirty_e: Some(false),
			dirty_f: Some(false),
			dirty_g: Some(false),
			dirty_h: Some(false),
		},
	)
	.await?;

	Ok(XmlImportResult {
		skipped: false,
		case_id: Some(case_id.to_string()),
		case_number: Some(safety_report_id),
		case_version: Some(i64::from(next_version)),
		xml_key: None,
		parsed_json_id: Some(version_id.to_string()),
	})
}

#[cfg(test)]
mod tests {
	use super::read_import_message_header;
	use crate::import_sections::shared::MessageHeaderExtract;

	fn header(number: &str, date: &str) -> MessageHeaderExtract {
		MessageHeaderExtract {
			message_number: Some(number.to_string()),
			message_sender: Some("sender".to_string()),
			message_receiver: Some("receiver".to_string()),
			message_date: Some(date.to_string()),
			batch_number: None,
			batch_sender: None,
			batch_receiver: None,
		}
	}

	#[test]
	fn header_reader_does_not_duplicate_c_1_1_import_check() {
		read_import_message_header(&header("message-id", "20260809000000"))
			.expect("read header");
	}

	#[test]
	fn reads_complete_message_header() {
		read_import_message_header(&header("case-id", "20260809000000"))
			.expect("read header");
	}
}
