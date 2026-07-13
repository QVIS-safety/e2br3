use crate::ctx::Ctx;
use crate::model::audit::{CaseVersionBmc, CaseVersionForCreate};
use crate::model::case::{CaseBmc, CaseForCreate, CaseForUpdate};
use crate::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use crate::model::store::set_full_context_dbx;
use crate::model::{self, ModelManager};
use crate::xml::error::Error;
use crate::xml::import_runtime::{c, d, e, f, g, h, shared};
use crate::xml::types::XmlImportResult;
use crate::xml::{parse_e2b_xml, Result};
use serde_json::json;

#[derive(Debug, Clone, Copy, Default)]
pub struct CImportSettings {
	pub update_date_of_creation: bool,
	pub update_most_recent_info_date: bool,
	pub update_report_first_received_date: bool,
	pub apply_sender_info_to_imported_cases: bool,
	pub apply_default_values_to_imported_r2_cases: bool,
	pub selected_sender_presave_id: Option<sqlx::types::Uuid>,
}

#[derive(Debug, Clone)]
pub struct XmlImportRequest {
	pub xml: Vec<u8>,
	pub filename: Option<String>,
	pub skip_validation: bool,
	pub c_settings: CImportSettings,
	pub product_presave_id: Option<sqlx::types::Uuid>,
	pub product_id: Option<String>,
}

pub fn extract_safety_report_id_from_xml(xml: &[u8]) -> Result<String> {
	shared::extract_safety_report_id(xml)
}

pub async fn import_e2b_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	import_e2b_xml_unvalidated(ctx, mm, req).await
}

pub async fn import_e2b_xml_unvalidated(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	let mm = mm.new_with_txn()?;
	let parsed = parse_e2b_xml(&req.xml)?;
	let safety_report_id_raw = shared::extract_safety_report_id(&req.xml)?;
	let safety_report_id = shared::clamp_str(
		Some(safety_report_id_raw),
		100,
		"safety_report_identification.safety_report_id",
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.1.REQUIRED: safety report identifier missing".to_string(),
		line: None,
		column: None,
	})?;
	let header_extract = shared::extract_message_header(&req.xml).ok();
	let next_version = {
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
		let sql = "select max(version) from safety_report_identification where safety_report_id = $1";
		let max_version: (Option<i32>,) = dbx
			.fetch_one(sqlx::query_as(sql).bind(&safety_report_id))
			.await
			.map_err(model::Error::from)?;
		dbx.commit_txn().await.map_err(model::Error::from)?;
		max_version.0.unwrap_or(0) + 1
	};

	let case_id = CaseBmc::create(
		ctx,
		&mm,
		CaseForCreate {
			organization_id: ctx.organization_id(),
			dg_prd_key: req.product_id.clone(),
			status: Some("draft".to_string()),
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
		},
	)
	.await?;

	if let Some(ref header) = header_extract {
		let message_number = header
			.message_number
			.clone()
			.unwrap_or_else(|| safety_report_id.clone());
		let message_number =
			shared::make_import_message_number(&message_number, case_id);
		let message_sender = header
			.message_sender
			.clone()
			.or_else(|| header.batch_sender.clone());
		let message_receiver = header
			.message_receiver
			.clone()
			.or_else(|| header.batch_receiver.clone());
		let message_date = header
			.message_date
			.clone()
			.or_else(|| header.batch_transmission.clone())
			.and_then(shared::normalize_message_date);
		let (msg_sender, msg_receiver, msg_date) = (
			message_sender.clone(),
			message_receiver.clone(),
			message_date.clone(),
		);
		if let (Some(message_sender), Some(message_receiver), Some(message_date)) =
			(msg_sender, msg_receiver, msg_date)
		{
			let has_header = MessageHeaderBmc::get_by_case(ctx, &mm, case_id)
				.await
				.is_ok();
			if !has_header {
				MessageHeaderBmc::create(
					ctx,
					&mm,
					MessageHeaderForCreate {
						case_id,
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
				&mm,
				case_id,
				MessageHeaderForUpdate {
					batch_number: header.batch_number.clone(),
					batch_sender_identifier: header.batch_sender.clone(),
					batch_receiver_identifier: header.batch_receiver.clone(),
					batch_transmission_date: None,
					message_number: None,
					message_sender_identifier: None,
					message_receiver_identifier: None,
					message_date: None,
				},
			)
			.await?;
		} else {
			tracing::warn!(
				message_sender = ?message_sender,
				message_receiver = ?message_receiver,
				message_date = ?message_date,
				"message header incomplete; skipping create"
			);
		}
	}

	c::import_section_c(
		ctx,
		&mm,
		&req.xml,
		case_id,
		&safety_report_id,
		next_version,
		header_extract.as_ref(),
		&req.c_settings,
	)
	.await?;
	d::import_section_d(ctx, &mm, &req.xml, case_id).await?;
	h::import_section_h(ctx, &mm, &req.xml, case_id).await?;

	let snapshot = json!({
		"parsed": parsed.json,
		"raw_xml": String::from_utf8_lossy(&req.xml),
	});

	let reaction_map = e::import_section_e(ctx, &mm, &req.xml, case_id).await?;
	f::import_section_f(ctx, &mm, &req.xml, case_id).await?;
	g::import_section_g(
		ctx,
		&mm,
		&req.xml,
		case_id,
		&reaction_map,
		req.product_presave_id,
	)
	.await?;

	let version_id = match CaseVersionBmc::create(
		ctx,
		&mm,
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
		&mm,
		case_id,
		CaseForUpdate {
			raw_xml: Some(req.xml.to_vec()),
			dg_prd_key: req.product_id,
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
		case_id: Some(case_id.to_string()),
		case_number: Some(safety_report_id),
		case_version: Some(i64::from(next_version)),
		xml_key: None,
		parsed_json_id: Some(version_id.to_string()),
	})
}
