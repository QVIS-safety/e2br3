use super::super::common::{
	bool_field, direct_page_projection_response, direct_section_response,
	explicit_null_model_fields, i32_field, json,
	mark_editor_validation_summary_stale, next_child_sequence, optional_row_object,
	query_authorities_csv, reject_unknown_row_keys, string_field, uuid_eq, BTreeMap,
	CaseBmc, CaseEditorCiCaseDto, CaseEditorCiDocumentDto,
	CaseEditorCiLinkedReportDto, CaseEditorCiOtherIdentifierDto,
	CaseEditorCiRowsDto, CaseEditorCiSafetyReportDto, CaseEditorCiSourceDocumentDto,
	CaseEditorDirectSectionResponse, CaseEditorPagePatchRequest,
	CaseEditorPageProjectionQuery, CaseEditorPageProjectionResponse, CaseForUpdate,
	CtxW, Deserialize, DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter,
	DocumentsHeldBySenderForCreate, DocumentsHeldBySenderForUpdate, Error, Json,
	LinkedReportNumberBmc, LinkedReportNumberFilter, LinkedReportNumberForCreate,
	LinkedReportNumberForUpdate, ListOptions, Map, ModelManager,
	OtherCaseIdentifierBmc, OtherCaseIdentifierFilter, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate, PatchValue, Path, Query, Result,
	SafetyReportIdentificationBmc, SafetyReportIdentificationForUpdate,
	SourceDocumentBmc, SourceDocumentFilter, SourceDocumentForCreate,
	SourceDocumentForUpdate, State, Uuid, Value,
};
use super::apply_editor_direct_page_patch;

const CI_SAFETY_REPORT_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("safety_report_id", &["safetyReportId"]),
	("version", &["version"]),
	("transmission_date", &["transmissionDate"]),
	("report_type", &["reportType"]),
	(
		"date_first_received_from_source",
		&["dateFirstReceivedFromSource"],
	),
	(
		"date_of_most_recent_information",
		&["dateOfMostRecentInformation"],
	),
	("fulfil_expedited_criteria", &["fulfilExpeditedCriteria"]),
	(
		"fulfil_expedited_criteria_null_flavor",
		&["fulfilExpeditedCriteriaNullFlavor"],
	),
	("local_criteria_report_type", &["localCriteriaReportType"]),
	(
		"combination_product_report_indicator",
		&["combinationProductReportIndicator"],
	),
	(
		"combination_product_report_indicator_null_flavor",
		&["combinationProductReportIndicatorNullFlavor"],
	),
	("worldwide_unique_id", &["worldwideUniqueId"]),
	("first_sender_type", &["firstSenderType"]),
	(
		"additional_documents_available",
		&["additionalDocumentsAvailable"],
	),
	(
		"other_case_identifiers_exist",
		&["otherCaseIdentifiersExist"],
	),
	(
		"other_case_identifiers_exist_null_flavor",
		&["otherCaseIdentifiersExistNullFlavor"],
	),
	("nullification_code", &["nullificationAmendmentCode"]),
	("nullification_reason", &["nullificationReason"]),
	("receiver_organization", &["receiverOrganization"]),
];
const CI_CASE_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("report_year", &["reportYear"]),
	("fda_report_type", &["fdaReportType"]),
	("mfds_report_type", &["mfdsReportType"]),
];
const CI_DOCUMENT_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("title", &["documentDescription"]),
	("document_base64", &["includedDocument"]),
	("file_name", &["fileName"]),
	("media_type", &["mediaType"]),
	("representation", &["representation"]),
	("compression", &["compression"]),
];
const CI_SOURCE_DOCUMENT_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("source_document_name", &["sourceDocumentName"]),
	("source_document_base64", &["sourceDocumentBase64"]),
	("source_document_media_type", &["sourceDocumentMediaType"]),
];
pub(super) async fn load_editor_ci_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let case_fields = CaseEditorCiCaseDto {
		report_year: case.report_year,
		fda_report_type: case.fda_report_type,
		mfds_report_type: case.mfds_report_type,
	};
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(CaseEditorCiSafetyReportDto {
				id: entity.id,
				safety_report_id: entity.safety_report_id,
				safety_report_version: entity.version,
				transmission_date: entity.transmission_date,
				report_type: entity.report_type,
				date_first_received_from_source: entity
					.date_first_received_from_source,
				date_of_most_recent_information: entity
					.date_of_most_recent_information,
				fulfil_expedited_criteria: entity.fulfil_expedited_criteria,
				fulfil_expedited_criteria_null_flavor: entity
					.fulfil_expedited_criteria_null_flavor,
				local_criteria_report_type: entity.local_criteria_report_type,
				combination_product_report_indicator: entity
					.combination_product_report_indicator,
				combination_product_report_indicator_null_flavor: entity
					.combination_product_report_indicator_null_flavor,
				worldwide_unique_id: entity.worldwide_unique_id,
				first_sender_type: entity.first_sender_type,
				additional_documents_available: entity
					.additional_documents_available,
				other_case_identifiers_exist: entity.other_case_identifiers_exist,
				other_case_identifiers_exist_null_flavor: entity
					.other_case_identifiers_exist_null_flavor,
				nullification_amendment_code: entity.nullification_code,
				nullification_reason: entity.nullification_reason,
			}),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let other_case_identifiers = OtherCaseIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![OtherCaseIdentifierFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let linked_reports = LinkedReportNumberBmc::list(
		ctx,
		mm,
		Some(vec![LinkedReportNumberFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let documents_held_by_sender = DocumentsHeldBySenderBmc::list(
		ctx,
		mm,
		Some(vec![DocumentsHeldBySenderFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let source_documents = SourceDocumentBmc::list(
		ctx,
		mm,
		Some(vec![SourceDocumentFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!(CaseEditorCiRowsDto {
		case: case_fields,
		safety_report_identification,
		other_case_identifiers: other_case_identifiers
			.into_iter()
			.map(|row| CaseEditorCiOtherIdentifierDto {
				id: row.id,
				source: row.source_of_identifier,
				case_identifier: row.case_identifier,
				sequence_number: row.sequence_number,
				deleted: row.deleted,
			})
			.collect(),
		linked_reports: linked_reports
			.into_iter()
			.map(|row| CaseEditorCiLinkedReportDto {
				id: row.id,
				linked_report_number: row.linked_report_number,
				sequence_number: row.sequence_number,
				deleted: row.deleted,
			})
			.collect(),
		documents_held_by_sender: documents_held_by_sender
			.into_iter()
			.map(|row| CaseEditorCiDocumentDto {
				id: row.id,
				document_description: row.title,
				included_document: row.document_base64,
				file_name: row.file_name,
				media_type: row.media_type,
				representation: row.representation,
				compression: row.compression,
				sequence_number: row.sequence_number,
				deleted: row.deleted,
			})
			.collect(),
		source_documents: source_documents
			.into_iter()
			.map(|row| CaseEditorCiSourceDocumentDto {
				id: row.id,
				source_document_name: row.source_document_name,
				source_document_base64: row.source_document_base64,
				source_document_media_type: row.source_document_media_type,
				sequence_number: row.sequence_number,
			})
			.collect(),
	}))
}

pub async fn get_editor_ci(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/CI",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_ci_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/editor/pages/CI
pub async fn get_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/CI",
		move |ctx, mm| {
			Box::pin(async move {
				let projection = direct_page_projection_response(
					ctx,
					mm,
					case_id,
					"CI",
					query_authorities_csv(&query)?,
					load_editor_ci_data(ctx, mm, case_id).await?,
				)
				.await?;
				Ok((axum::http::StatusCode::OK, Json(projection)))
			})
		},
	)
	.await
}

#[derive(Deserialize)]
pub(super) struct CiDatePatchValue {
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub(super) value: Option<sqlx::types::time::Date>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CiCaseRowPatch {
	report_year: Option<String>,
	fda_report_type: Option<String>,
	mfds_report_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CiDocumentRowPatch {
	#[serde(default)]
	id: Option<Uuid>,
	#[serde(default)]
	document_description: Option<String>,
	#[serde(default)]
	included_document: Option<String>,
	#[serde(default)]
	file_name: Option<String>,
	#[serde(default)]
	media_type: Option<String>,
	#[serde(default)]
	representation: Option<String>,
	#[serde(default)]
	compression: Option<String>,
	#[serde(default)]
	sequence_number: Option<i32>,
	#[serde(default)]
	deleted: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CiOtherIdentifierRowPatch {
	#[serde(default)]
	id: Option<Uuid>,
	#[serde(default)]
	source: Option<String>,
	#[serde(default)]
	case_identifier: Option<String>,
	#[serde(default)]
	sequence_number: Option<i32>,
	#[serde(default)]
	deleted: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CiLinkedReportRowPatch {
	#[serde(default)]
	id: Option<Uuid>,
	#[serde(default)]
	linked_report_number: Option<String>,
	#[serde(default)]
	sequence_number: Option<i32>,
	#[serde(default)]
	deleted: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CiSourceDocumentRowPatch {
	#[serde(default)]
	deleted: Option<bool>,
	#[serde(default)]
	id: Option<Uuid>,
	#[serde(default)]
	source_document_name: Option<String>,
	#[serde(default)]
	source_document_base64: Option<String>,
	#[serde(default)]
	source_document_media_type: Option<String>,
	#[serde(default)]
	sequence_number: Option<i32>,
}

fn ci_row_error(owner: &str, err: serde_json::Error) -> Error {
	Error::BadRequest {
		message: format!("CI.{owner} has an invalid row payload: {err}"),
	}
}

pub(super) async fn apply_ci_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	if rows.contains_key("messageHeader") {
		return Err(Error::BadRequest {
			message: "CI.messageHeader row patches are not supported".to_string(),
		});
	}
	reject_unknown_row_keys(
		"CI",
		rows,
		&[
			"case",
			"messageHeader",
			"safetyReportIdentification",
			"documentsHeldBySender",
			"otherCaseIdentifiers",
			"linkedReports",
			"sourceDocuments",
		],
	)?;

	if let Some(row) = optional_row_object("CI", rows, "safetyReportIdentification")?
	{
		fn patch_string(
			row: &Map<String, Value>,
			key: &str,
		) -> Result<PatchValue<String>> {
			match row.get(key) {
				None => Ok(PatchValue::Missing),
				Some(Value::Null) => Ok(PatchValue::Null),
				Some(Value::String(value)) if value.trim().is_empty() => {
					Ok(PatchValue::Missing)
				}
				Some(Value::String(value)) => Ok(PatchValue::Value(value.clone())),
				Some(_) => Err(Error::BadRequest {
					message: format!(
						"CI.safetyReportIdentification.{key} must be a string or null"
					),
				}),
			}
		}
		fn patch_bool(
			row: &Map<String, Value>,
			key: &str,
		) -> Result<PatchValue<bool>> {
			match row.get(key) {
				None => Ok(PatchValue::Missing),
				Some(Value::Null) => Ok(PatchValue::Null),
				Some(Value::Bool(value)) => Ok(PatchValue::Value(*value)),
				Some(_) => Err(Error::BadRequest {
					message: format!(
						"CI.safetyReportIdentification.{key} must be a boolean or null"
					),
				}),
			}
		}
		fn date_text(row: &Map<String, Value>, key: &str) -> Result<Option<String>> {
			let value = row.get(key).cloned().unwrap_or(Value::Null);
			if value.as_str().is_some_and(|value| value.trim().is_empty()) {
				return Ok(None);
			}
			if let Some(value) = value.as_str() {
				return Ok(Some(value.to_string()));
			}
			serde_json::from_value::<CiDatePatchValue>(json!({ "value": value }))
				.map(|value| {
					value.value.map(|date| {
						format!(
							"{:04}{:02}{:02}",
							date.year(),
							u8::from(date.month()),
							date.day()
						)
					})
				})
				.map_err(|err| Error::BadRequest {
					message: format!(
						"CI.safetyReportIdentification.{key} must be an E2B date or null: {err}"
					),
				})
		}

		let clear_fields =
			explicit_null_model_fields(row, CI_SAFETY_REPORT_PATCH_FIELDS);
		SafetyReportIdentificationBmc::update_by_case_patch(
			ctx,
			mm,
			case_id,
			SafetyReportIdentificationForUpdate {
				safety_report_id: string_field(row, &["safetyReportId"]),
				version: i32_field(row, &["version"]),
				transmission_date: string_field(row, &["transmissionDate"]),
				report_type: patch_string(row, "reportType")?,
				date_first_received_from_source: date_text(
					row,
					"dateFirstReceivedFromSource",
				)?,
				date_of_most_recent_information: date_text(
					row,
					"dateOfMostRecentInformation",
				)?,
				fulfil_expedited_criteria: patch_bool(
					row,
					"fulfilExpeditedCriteria",
				)?,
				fulfil_expedited_criteria_null_flavor: string_field(
					row,
					&["fulfilExpeditedCriteriaNullFlavor"],
				),
				local_criteria_report_type: patch_string(
					row,
					"localCriteriaReportType",
				)?,
				combination_product_report_indicator: patch_string(
					row,
					"combinationProductReportIndicator",
				)?,
				combination_product_report_indicator_null_flavor: string_field(
					row,
					&["combinationProductReportIndicatorNullFlavor"],
				),
				worldwide_unique_id: string_field(row, &["worldwideUniqueId"]),
				first_sender_type: string_field(row, &["firstSenderType"]),
				additional_documents_available: bool_field(
					row,
					&["additionalDocumentsAvailable"],
				),
				other_case_identifiers_exist: bool_field(
					row,
					&["otherCaseIdentifiersExist"],
				),
				other_case_identifiers_exist_null_flavor: string_field(
					row,
					&["otherCaseIdentifiersExistNullFlavor"],
				),
				nullification_code: string_field(
					row,
					&["nullificationAmendmentCode"],
				),
				nullification_reason: string_field(row, &["nullificationReason"]),
				receiver_organization: string_field(row, &["receiverOrganization"]),
			},
			&clear_fields,
		)
		.await?;
	}

	if let Some(value) = rows.get("case") {
		let patch = serde_json::from_value::<CiCaseRowPatch>(value.clone())
			.map_err(|err| ci_row_error("case", err))?;
		let clear_fields = explicit_null_model_fields(
			value.as_object().expect("validated CI case row"),
			CI_CASE_PATCH_FIELDS,
		);
		lib_core::model::update_uuid_patch::<CaseBmc, _>(
			ctx,
			mm,
			case_id,
			CaseForUpdate {
				report_year: patch.report_year,
				fda_report_type: patch.fda_report_type,
				mfds_report_type: patch.mfds_report_type,
				dirty_c: Some(true),
				..Default::default()
			},
			&clear_fields,
		)
		.await?;
	}

	if let Some(value) = rows.get("documentsHeldBySender") {
		let patches =
			serde_json::from_value::<Vec<CiDocumentRowPatch>>(value.clone())
				.map_err(|err| ci_row_error("documentsHeldBySender", err))?;
		let raw_patches = value.as_array().expect("validated CI document array");
		for (patch, raw_patch) in patches.into_iter().zip(raw_patches) {
			if patch.id.is_none() && patch.deleted == Some(true) {
				continue;
			}
			if let Some(id) = patch.id {
				let current = DocumentsHeldBySenderBmc::get(ctx, mm, id).await?;
				if current.case_id != case_id {
					return Err(Error::BadRequest {
						message: format!("CI.documentsHeldBySender row '{id}' belongs to another case"),
					});
				}
				if patch.deleted == Some(true) {
					DocumentsHeldBySenderBmc::delete(ctx, mm, id).await?;
					continue;
				}
				if patch.deleted == Some(false) && current.deleted {
					DocumentsHeldBySenderBmc::restore(ctx, mm, id).await?;
				}
				let clear_fields = explicit_null_model_fields(
					raw_patch.as_object().expect("validated CI document row"),
					CI_DOCUMENT_PATCH_FIELDS,
				);
				lib_core::model::update_uuid_patch::<DocumentsHeldBySenderBmc, _>(
					ctx,
					mm,
					id,
					DocumentsHeldBySenderForUpdate {
						title: patch.document_description,
						document_base64: patch.included_document,
						file_name: patch.file_name,
						media_type: patch.media_type,
						representation: patch.representation,
						compression: patch.compression,
						sequence_number: patch.sequence_number,
					},
					&clear_fields,
				)
				.await?;
			} else {
				if patch.deleted == Some(true) {
					return Err(Error::BadRequest {
						message:
							"a new CI.documentsHeldBySender row cannot be deleted"
								.to_string(),
					});
				}
				DocumentsHeldBySenderBmc::create(
					ctx,
					mm,
					DocumentsHeldBySenderForCreate {
						case_id,
						title: patch.document_description,
						document_base64: patch.included_document,
						file_name: patch.file_name,
						media_type: patch.media_type,
						representation: patch.representation,
						compression: patch.compression,
						sequence_number: patch.sequence_number.unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"documents_held_by_sender",
								"case_id",
								case_id,
								true,
							)
							.await?,
						),
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("otherCaseIdentifiers") {
		let patches =
			serde_json::from_value::<Vec<CiOtherIdentifierRowPatch>>(value.clone())
				.map_err(|err| ci_row_error("otherCaseIdentifiers", err))?;
		for patch in patches {
			if let Some(id) = patch.id {
				let current = OtherCaseIdentifierBmc::get(ctx, mm, id).await?;
				if current.case_id != case_id {
					return Err(Error::BadRequest {
						message: format!("CI.otherCaseIdentifiers row '{id}' belongs to another case"),
					});
				}
				if patch.deleted == Some(true) {
					OtherCaseIdentifierBmc::delete(ctx, mm, id).await?;
					continue;
				}
				if patch.deleted == Some(false) && current.deleted {
					OtherCaseIdentifierBmc::restore(ctx, mm, id).await?;
				}
				OtherCaseIdentifierBmc::update(
					ctx,
					mm,
					id,
					OtherCaseIdentifierForUpdate {
						source_of_identifier: patch.source,
						case_identifier: patch.case_identifier,
					},
				)
				.await?;
			} else {
				if patch.deleted == Some(true) {
					continue;
				}
				let source = patch.source.unwrap_or_default();
				let case_identifier = patch.case_identifier.unwrap_or_default();
				OtherCaseIdentifierBmc::create(
					ctx,
					mm,
					OtherCaseIdentifierForCreate {
						case_id,
						sequence_number: patch.sequence_number.unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"other_case_identifiers",
								"case_id",
								case_id,
								true,
							)
							.await?,
						),
						source_of_identifier: source,
						case_identifier,
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("linkedReports") {
		let patches =
			serde_json::from_value::<Vec<CiLinkedReportRowPatch>>(value.clone())
				.map_err(|err| ci_row_error("linkedReports", err))?;
		for patch in patches {
			if let Some(id) = patch.id {
				let current = LinkedReportNumberBmc::get(ctx, mm, id).await?;
				if current.case_id != case_id {
					return Err(Error::BadRequest {
						message: format!(
							"CI.linkedReports row '{id}' belongs to another case"
						),
					});
				}
				if patch.deleted == Some(true) {
					LinkedReportNumberBmc::delete(ctx, mm, id).await?;
					continue;
				}
				if patch.deleted == Some(false) && current.deleted {
					LinkedReportNumberBmc::restore(ctx, mm, id).await?;
				}
				LinkedReportNumberBmc::update(
					ctx,
					mm,
					id,
					LinkedReportNumberForUpdate {
						linked_report_number: patch.linked_report_number,
					},
				)
				.await?;
			} else {
				let linked_report_number =
					patch
						.linked_report_number
						.ok_or_else(|| Error::BadRequest {
							message:
								"CI.linkedReports[].linkedReportNumber is required"
									.to_string(),
						})?;
				LinkedReportNumberBmc::create(
					ctx,
					mm,
					LinkedReportNumberForCreate {
						case_id,
						sequence_number: patch.sequence_number.unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"linked_report_numbers",
								"case_id",
								case_id,
								true,
							)
							.await?,
						),
						linked_report_number,
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("sourceDocuments") {
		let patches =
			serde_json::from_value::<Vec<CiSourceDocumentRowPatch>>(value.clone())
				.map_err(|err| ci_row_error("sourceDocuments", err))?;
		let raw_patches = value
			.as_array()
			.expect("validated CI source document array");
		for (patch, raw_patch) in patches.into_iter().zip(raw_patches) {
			if patch.id.is_none() && patch.deleted == Some(true) {
				continue;
			}
			if let Some(id) = patch.id {
				let current = SourceDocumentBmc::get(ctx, mm, id).await?;
				if current.case_id != case_id {
					return Err(Error::BadRequest {
						message: format!(
							"CI.sourceDocuments row '{id}' belongs to another case"
						),
					});
				}
				let clear_fields = explicit_null_model_fields(
					raw_patch
						.as_object()
						.expect("validated CI source document row"),
					CI_SOURCE_DOCUMENT_PATCH_FIELDS,
				);
				if patch.deleted == Some(true) {
					SourceDocumentBmc::delete(ctx, mm, id).await?;
					continue;
				}
				lib_core::model::update_uuid_patch::<SourceDocumentBmc, _>(
					ctx,
					mm,
					id,
					SourceDocumentForUpdate {
						source_document_name: patch.source_document_name,
						source_document_base64: patch.source_document_base64,
						source_document_media_type: patch.source_document_media_type,
						sequence_number: patch.sequence_number,
					},
					&clear_fields,
				)
				.await?;
			} else {
				SourceDocumentBmc::create(
					ctx,
					mm,
					SourceDocumentForCreate {
						case_id,
						source_document_name: patch.source_document_name,
						source_document_base64: patch.source_document_base64,
						source_document_media_type: patch.source_document_media_type,
						sequence_number: patch.sequence_number.unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"source_documents",
								"case_id",
								case_id,
								false,
							)
							.await?,
						),
					},
				)
				.await?;
			}
		}
	}

	Ok(())
}

/// PATCH /api/cases/{case_id}/editor/pages/CI
pub async fn patch_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/CI",
		move |ctx, mm| {
			Box::pin(patch_editor_ci_page_projection_authorized(
				ctx, mm, case_id, request,
			))
		},
	)
	.await
}

async fn patch_editor_ci_page_projection_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	request: CaseEditorPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let has_rows = !request.rows.is_empty();
	let requested_authorities =
		apply_editor_direct_page_patch(ctx, mm, case_id, "CI", request).await?;
	if has_rows {
		mark_editor_validation_summary_stale(
			ctx,
			mm,
			case_id,
			requested_authorities.clone(),
		)
		.await?;
	}
	let projection = direct_page_projection_response(
		ctx,
		mm,
		case_id,
		"CI",
		requested_authorities,
		load_editor_ci_data(ctx, mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}
