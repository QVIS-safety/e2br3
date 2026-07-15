use super::common::*;

async fn load_editor_ci_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let case = PublicCaseView::from(CaseBmc::get(ctx, mm, case_id).await?);
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let message_header = match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await
	{
		Ok(entity) => Some(entity),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
		Err(err) => return Err(err.into()),
	};
	let receiver_information =
		ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;
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

	Ok(json!({
		"case": case,
		"safetyReportIdentification": safety_report_identification,
		"messageHeader": message_header,
		"receiverInfo": receiver_information,
		"otherCaseIdentifiers": other_case_identifiers,
		"linkedReports": linked_reports,
		"documentsHeldBySender": documents_held_by_sender,
	}))
}

pub async fn get_editor_ci(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	require_permission(&ctx, MESSAGE_HEADER_READ)?;
	require_permission(&ctx, RECEIVER_READ)?;
	require_permission(&ctx, CASE_IDENTIFIER_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_ci_data(&ctx, &mm, case_id).await?,
	))
}

/// GET /api/cases/{case_id}/editor/pages/CI
pub async fn get_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"CI",
		query_authorities_csv(&query)?,
		load_editor_ci_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

fn patch_string_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<PatchValue<String>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(PatchValue::Missing);
	};
	if value.is_null() {
		return Ok(PatchValue::Null);
	}
	let Some(value) = value.as_str() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a string or null"),
		});
	};
	Ok(PatchValue::Value(value.trim().to_string()))
}

fn patch_bool_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<PatchValue<bool>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(PatchValue::Missing);
	};
	if value.is_null() {
		return Ok(PatchValue::Null);
	}
	let Some(value) = value.as_bool() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a boolean or null"),
		});
	};
	Ok(PatchValue::Value(value))
}

fn patch_optional_string_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<Option<String>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(None);
	};
	if value.is_null() {
		return Ok(None);
	}
	let Some(value) = value.as_str() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a string or null"),
		});
	};
	Ok(Some(value.trim().to_string()))
}

fn patch_optional_bool_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<Option<bool>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(None);
	};
	if value.is_null() {
		return Ok(None);
	}
	let Some(value) = value.as_bool() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a boolean or null"),
		});
	};
	Ok(Some(value))
}

/// PATCH /api/cases/{case_id}/editor/pages/CI
pub async fn patch_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;
	validate_direct_changes("CI", &request.changes)?;

	let mut update = SafetyReportIdentificationForUpdate {
		safety_report_id: None,
		version: None,
		transmission_date: None,
		report_type: PatchValue::Missing,
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: PatchValue::Missing,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: PatchValue::Missing,
		combination_product_report_indicator: PatchValue::Missing,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
	};

	for (field, patch) in &request.changes {
		match field.as_str() {
			"reportType" => {
				update.report_type = patch_string_value(field, patch)?;
			}
			"fulfilExpeditedCriteria" => {
				update.fulfil_expedited_criteria = patch_bool_value(field, patch)?;
			}
			"fulfilExpeditedCriteriaNullFlavor" => {
				update.fulfil_expedited_criteria_null_flavor =
					patch_optional_string_value(field, patch)?;
			}
			"localCriteriaReportType" => {
				update.local_criteria_report_type =
					patch_string_value(field, patch)?;
			}
			"combinationProductReportIndicator" => {
				update.combination_product_report_indicator =
					patch_string_value(field, patch)?;
			}
			"combinationProductReportIndicatorNullFlavor" => {
				update.combination_product_report_indicator_null_flavor =
					patch_optional_string_value(field, patch)?;
			}
			"otherCaseIdentifiersExist" => {
				update.other_case_identifiers_exist =
					patch_optional_bool_value(field, patch)?;
			}
			"otherCaseIdentifiersExistNullFlavor" => {
				update.other_case_identifiers_exist_null_flavor =
					patch_optional_string_value(field, patch)?;
			}
			_ => {
				return Err(Error::BadRequest {
					message: format!("unknown CI field '{field}'"),
				});
			}
		}
	}
	if !request.rows.is_empty() {
		return Err(Error::BadRequest {
			message: "CI row patch operations are not implemented in this slice"
				.to_string(),
		});
	}

	if !request.changes.is_empty() {
		SafetyReportIdentificationBmc::update_by_case(&ctx, &mm, case_id, update)
			.await?;
		refresh_editor_validation_cache(
			&ctx,
			&mm,
			case_id,
			requested_authorities.clone(),
		)
		.await?;
	}
	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"CI",
		requested_authorities,
		load_editor_ci_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub async fn patch_editor_rp_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "RP", request).await
}

pub async fn patch_editor_sd_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SD", request).await
}

pub async fn patch_editor_lr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "LR", request).await
}

pub async fn patch_editor_si_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SI", request).await
}

pub async fn patch_editor_dm_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "DM", request).await
}

pub async fn patch_editor_nr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "NR", request).await
}

async fn patch_direct_page_projection(
	mm: ModelManager,
	ctx_w: CtxW,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;
	validate_direct_changes(page_id, &request.changes)?;
	validate_direct_rows(page_id, &request.rows)?;

	if !request.changes.is_empty() {
		apply_direct_page_changes_patch(
			&ctx,
			&mm,
			case_id,
			page_id,
			&request.changes,
		)
		.await?;
	}

	if !request.rows.is_empty() {
		apply_direct_page_rows_patch(&ctx, &mm, case_id, page_id, &request.rows)
			.await?;
	}

	if !request.changes.is_empty() || !request.rows.is_empty() {
		refresh_editor_validation_cache(
			&ctx,
			&mm,
			case_id,
			requested_authorities.clone(),
		)
		.await?;
	}

	let data = match page_id {
		"RP" => load_editor_rp_data(&ctx, &mm, case_id).await?,
		"SD" => load_editor_sd_data(&ctx, &mm, case_id).await?,
		"LR" => load_editor_lr_data(&ctx, &mm, case_id).await?,
		"SI" => load_editor_si_data(&ctx, &mm, case_id).await?,
		"DM" => load_editor_dm_data(&ctx, &mm, case_id).await?,
		"NR" => load_editor_nr_data(&ctx, &mm, case_id).await?,
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported direct page '{page_id}'"),
			})
		}
	};
	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		page_id,
		requested_authorities,
		data,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn apply_direct_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	match page_id {
		"RP" => apply_rp_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SD" => apply_sd_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"LR" => apply_lr_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SI" => apply_si_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"DM" => apply_dm_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"NR" => apply_nr_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		_ => Err(Error::BadRequest {
			message: format!("unsupported direct page '{page_id}'"),
		}),
	}
}

async fn apply_direct_page_changes_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<()> {
	let rows = match page_id {
		"RP" => row_array_payload_from_changes(
			page_id,
			"primarySources",
			changes,
			&[
				("reporterTitle", "reporterTitle"),
				("reporterGivenName", "reporterGivenName"),
				("reporterMiddleName", "reporterMiddleName"),
				("reporterFamilyName", "reporterFamilyName"),
				("reporterNameNullFlavor", "reporterNameNullFlavor"),
				("reporterOrganization", "reporterOrganization"),
				("reporterCountry", "reporterCountry"),
				("reporterAddressNullFlavor", "reporterAddressNullFlavor"),
				("qualification", "qualification"),
				("qualificationNullFlavor", "qualificationNullFlavor"),
				("qualificationKr1", "qualificationKr1"),
			],
		)?,
		"SD" => direct_sd_rows_from_changes(page_id, changes)?,
		"LR" => row_array_payload_from_changes(
			page_id,
			"literatureReferences",
			changes,
			&[
				("literatureReference", "referenceText"),
				("referenceText", "referenceText"),
			],
		)?,
		"SI" => row_payload_from_changes(
			page_id,
			"studyInformation",
			changes,
			&[
				("studyName", "studyName"),
				("sponsorStudyNumber", "sponsorStudyNumber"),
				("studyTypeReaction", "studyTypeReaction"),
				("studyTypeReactionKr1", "studyTypeReactionKr1"),
			],
		)?,
		"DM" => row_payload_from_changes(
			page_id,
			"patientInformation",
			changes,
			&[
				("patientInitials", "patientInitials"),
				("patientGivenName", "patientGivenName"),
				("patientFamilyName", "patientFamilyName"),
				("patientSex", "sex"),
				("sex", "sex"),
			],
		)?,
		"NR" => row_payload_from_changes(
			page_id,
			"narrative",
			changes,
			&[
				("caseNarrative", "caseNarrative"),
				("reporterComments", "reporterComments"),
				("senderComments", "senderComments"),
			],
		)?,
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported direct page '{page_id}'"),
			})
		}
	};
	apply_direct_page_rows_patch(ctx, mm, case_id, page_id, &rows).await
}

fn direct_sd_rows_from_changes(
	page_id: &str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<BTreeMap<String, Value>> {
	let mut rows = BTreeMap::new();
	for (field, patch) in changes {
		let (row_key, target) = match field.as_str() {
			"senderType" => ("senderInformation", "senderType"),
			"senderOrganization" => ("senderInformation", "organizationName"),
			"senderDepartment" => ("senderInformation", "department"),
			"senderCountryCode" => ("senderInformation", "countryCode"),
			"messageNumber" => ("messageHeader", "messageNumber"),
			"messageSenderIdentifier" => {
				("messageHeader", "messageSenderIdentifier")
			}
			"messageReceiverIdentifier" => {
				("messageHeader", "messageReceiverIdentifier")
			}
			"batchReceiverIdentifier" => {
				("messageHeader", "batchReceiverIdentifier")
			}
			"receiverOrganization" => ("receiverInformation", "organizationName"),
			"receiverCountryCode" => ("receiverInformation", "countryCode"),
			_ => {
				return Err(Error::BadRequest {
					message: format!("unknown {page_id} field '{field}'"),
				})
			}
		};
		let entry = rows
			.entry(row_key.to_string())
			.or_insert_with(|| Value::Object(serde_json::Map::new()));
		let Some(map) = entry.as_object_mut() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.{row_key} must be an object"),
			});
		};
		map.insert(target.to_string(), patch_json_value(patch));
	}
	Ok(rows)
}

async fn apply_rp_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(page_id, rows, &["primarySources"])?;
	let Some(source) = optional_first_row_object(page_id, rows, "primarySources")?
	else {
		return Ok(());
	};
	let update = PrimarySourceForUpdate {
		source_reporter_presave_id: uuid_field(
			source,
			&["sourceReporterPresaveId", "source_reporter_presave_id"],
		),
		reporter_title: string_field(source, &["reporterTitle", "reporter_title"]),
		reporter_given_name: string_field(
			source,
			&["reporterGivenName", "reporter_given_name"],
		),
		reporter_middle_name: string_field(
			source,
			&["reporterMiddleName", "reporter_middle_name"],
		),
		reporter_family_name: string_field(
			source,
			&["reporterFamilyName", "reporter_family_name"],
		),
		reporter_name_null_flavor: string_field(
			source,
			&["reporterNameNullFlavor", "reporter_name_null_flavor"],
		),
		organization: string_field(
			source,
			&["reporterOrganization", "organization"],
		),
		department: string_field(source, &["reporterDepartment", "department"]),
		street: string_field(source, &["reporterStreet", "street"]),
		city: string_field(source, &["reporterCity", "city"]),
		state: string_field(source, &["reporterState", "state"]),
		postcode: string_field(source, &["reporterPostcode", "postcode"]),
		telephone: string_field(source, &["reporterTelephone", "telephone"]),
		reporter_address_null_flavor: string_field(
			source,
			&["reporterAddressNullFlavor", "reporter_address_null_flavor"],
		),
		country_code: string_field(source, &["reporterCountry", "country_code"]),
		country_code_null_flavor: string_field(
			source,
			&["reporterCountryNullFlavor", "country_code_null_flavor"],
		),
		email: string_field(source, &["reporterEmail", "email"]),
		email_null_flavor: string_field(
			source,
			&["reporterEmailNullFlavor", "email_null_flavor"],
		),
		qualification: string_field(source, &["qualification"]),
		qualification_null_flavor: string_field(
			source,
			&["qualificationNullFlavor", "qualification_null_flavor"],
		),
		qualification_kr1: string_field(
			source,
			&["qualificationKr1", "qualification_kr1"],
		),
		primary_source_regulatory: string_field(
			source,
			&[
				"primarySourceForRegulatoryPurposes",
				"primary_source_regulatory",
			],
		),
	};
	if let Some(id) = uuid_field(source, &["id"]) {
		PrimarySourceBmc::update(ctx, mm, id, update).await?;
	} else {
		PrimarySourceBmc::create(
			ctx,
			mm,
			PrimarySourceForCreate {
				case_id,
				source_reporter_presave_id: update.source_reporter_presave_id,
				sequence_number: i32_field(
					source,
					&["sequenceNumber", "sequence_number"],
				)
				.unwrap_or(1),
				reporter_title: update.reporter_title,
				reporter_given_name: update.reporter_given_name,
				reporter_middle_name: update.reporter_middle_name,
				reporter_family_name: update.reporter_family_name,
				reporter_name_null_flavor: update.reporter_name_null_flavor,
				organization: update.organization,
				department: update.department,
				street: update.street,
				city: update.city,
				state: update.state,
				postcode: update.postcode,
				telephone: update.telephone,
				reporter_address_null_flavor: update.reporter_address_null_flavor,
				country_code: update.country_code,
				country_code_null_flavor: update.country_code_null_flavor,
				email: update.email,
				email_null_flavor: update.email_null_flavor,
				qualification: update.qualification,
				qualification_null_flavor: update.qualification_null_flavor,
				qualification_kr1: update.qualification_kr1,
				primary_source_regulatory: update.primary_source_regulatory,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_sd_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&[
			"safetyReportIdentification",
			"messageHeader",
			"senderInformation",
			"receiverInformation",
		],
	)?;
	if let Some(message_header) =
		optional_row_object(page_id, rows, "messageHeader")?
	{
		MessageHeaderBmc::update_by_case(
			ctx,
			mm,
			case_id,
			MessageHeaderForUpdate {
				batch_number: string_field(
					message_header,
					&["batchNumber", "batch_number"],
				),
				batch_sender_identifier: string_field(
					message_header,
					&["batchSenderIdentifier", "batch_sender_identifier"],
				),
				batch_receiver_identifier: string_field(
					message_header,
					&["batchReceiverIdentifier", "batch_receiver_identifier"],
				),
				batch_transmission_date: None,
				message_number: string_field(
					message_header,
					&["messageNumber", "message_number"],
				),
				message_sender_identifier: string_field(
					message_header,
					&["messageSenderIdentifier", "message_sender_identifier"],
				),
				message_receiver_identifier: string_field(
					message_header,
					&["messageReceiverIdentifier", "message_receiver_identifier"],
				),
				message_date: string_field(
					message_header,
					&["messageDate", "message_date"],
				),
			},
		)
		.await?;
	}
	if let Some(sender) = optional_row_object(page_id, rows, "senderInformation")? {
		let update = SenderInformationForUpdate {
			source_sender_presave_id: uuid_field(
				sender,
				&["sourceSenderPresaveId", "source_sender_presave_id"],
			),
			sender_type: string_field(sender, &["senderType", "sender_type"]),
			health_professional_type_kr1: string_field(
				sender,
				&["healthProfessionalTypeKr1", "health_professional_type_kr1"],
			),
			organization_name: string_field(
				sender,
				&["organizationName", "organization_name"],
			),
			department: string_field(sender, &["department"]),
			street_address: string_field(
				sender,
				&["streetAddress", "street_address"],
			),
			city: string_field(sender, &["city"]),
			state: string_field(sender, &["state"]),
			postcode: string_field(sender, &["postcode"]),
			country_code: string_field(sender, &["countryCode", "country_code"]),
			person_title: string_field(sender, &["personTitle", "person_title"]),
			person_given_name: string_field(
				sender,
				&["personGivenName", "person_given_name"],
			),
			person_middle_name: string_field(
				sender,
				&["personMiddleName", "person_middle_name"],
			),
			person_family_name: string_field(
				sender,
				&["personFamilyName", "person_family_name"],
			),
			telephone: string_field(sender, &["telephone"]),
			fax: string_field(sender, &["fax"]),
			email: string_field(sender, &["email"]),
		};
		if let Some(id) = uuid_field(sender, &["id"]) {
			SenderInformationBmc::update(ctx, mm, id, update).await?;
		} else {
			SenderInformationBmc::create(
				ctx,
				mm,
				SenderInformationForCreate {
					case_id,
					source_sender_presave_id: update.source_sender_presave_id,
					sender_type: update.sender_type,
					health_professional_type_kr1: update
						.health_professional_type_kr1,
					organization_name: update.organization_name,
					department: update.department,
					street_address: update.street_address,
					city: update.city,
					state: update.state,
					postcode: update.postcode,
					country_code: update.country_code,
					person_title: update.person_title,
					person_given_name: update.person_given_name,
					person_middle_name: update.person_middle_name,
					person_family_name: update.person_family_name,
					telephone: update.telephone,
					fax: update.fax,
					email: update.email,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn apply_lr_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(page_id, rows, &["literatureReferences"])?;
	let Some(reference) =
		optional_first_row_object(page_id, rows, "literatureReferences")?
	else {
		return Ok(());
	};
	let update = LiteratureReferenceForUpdate {
		reference_text: string_field(
			reference,
			&["referenceText", "reference_text"],
		),
		reference_text_null_flavor: string_field(
			reference,
			&["referenceTextNullFlavor", "reference_text_null_flavor"],
		),
		sequence_number: i32_field(
			reference,
			&["sequenceNumber", "sequence_number"],
		),
		document_base64: string_field(
			reference,
			&["documentBase64", "document_base64"],
		),
		media_type: string_field(reference, &["mediaType", "media_type"]),
		representation: string_field(reference, &["representation"]),
		compression: string_field(reference, &["compression"]),
	};
	if let Some(id) = uuid_field(reference, &["id"]) {
		LiteratureReferenceBmc::update(ctx, mm, id, update).await?;
	} else if let Some(reference_text) = update.reference_text {
		LiteratureReferenceBmc::create(
			ctx,
			mm,
			LiteratureReferenceForCreate {
				case_id,
				reference_text,
				reference_text_null_flavor: update.reference_text_null_flavor,
				sequence_number: update.sequence_number.unwrap_or(1),
				document_base64: update.document_base64,
				media_type: update.media_type,
				representation: update.representation,
				compression: update.compression,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_si_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["studyInformation", "studyRegistrationNumbers"],
	)?;
	let Some(study) = optional_row_object(page_id, rows, "studyInformation")? else {
		return Ok(());
	};
	let update = StudyInformationForUpdate {
		source_study_presave_id: uuid_field(
			study,
			&["sourceStudyPresaveId", "source_study_presave_id"],
		),
		study_name: string_field(study, &["studyName", "study_name"]),
		study_name_null_flavor: string_field(
			study,
			&["studyNameNullFlavor", "study_name_null_flavor"],
		),
		sponsor_study_number: string_field(
			study,
			&["sponsorStudyNumber", "sponsor_study_number"],
		),
		sponsor_study_number_null_flavor: string_field(
			study,
			&[
				"sponsorStudyNumberNullFlavor",
				"sponsor_study_number_null_flavor",
			],
		),
		study_type_reaction: string_field(
			study,
			&["studyTypeReaction", "study_type_reaction"],
		),
		study_type_reaction_kr1: string_field(
			study,
			&["studyTypeReactionKr1", "study_type_reaction_kr1"],
		),
		fda_ind_number_occurred: string_field(
			study,
			&["fdaIndNumberOccurred", "fda_ind_number_occurred"],
		),
		fda_pre_anda_number_occurred: string_field(
			study,
			&["fdaPreAndaNumberOccurred", "fda_pre_anda_number_occurred"],
		),
	};
	if let Some(id) = uuid_field(study, &["id"]) {
		StudyInformationBmc::update(ctx, mm, id, update).await?;
	} else {
		StudyInformationBmc::create(
			ctx,
			mm,
			StudyInformationForCreate {
				case_id,
				source_study_presave_id: update.source_study_presave_id,
				study_name: update.study_name,
				study_name_null_flavor: update.study_name_null_flavor,
				sponsor_study_number: update.sponsor_study_number,
				sponsor_study_number_null_flavor: update
					.sponsor_study_number_null_flavor,
				study_type_reaction: update.study_type_reaction,
				study_type_reaction_kr1: update.study_type_reaction_kr1,
				fda_ind_number_occurred: update.fda_ind_number_occurred,
				fda_pre_anda_number_occurred: update.fda_pre_anda_number_occurred,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_dm_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&[
			"patientInformation",
			"patientIdentifiers",
			"medicalHistoryEpisodes",
			"deathInfo",
			"reportedCauses",
			"autopsyCauses",
			"parentInfo",
			"parentMedicalHistory",
			"parentPastDrugs",
		],
	)?;
	let Some(patient) = optional_row_object(page_id, rows, "patientInformation")?
	else {
		return Ok(());
	};
	let update = PatientInformationForUpdate {
		patient_initials: string_field(
			patient,
			&["patientInitials", "patient_initials"],
		),
		patient_given_name: string_field(
			patient,
			&["patientGivenName", "patient_given_name"],
		),
		patient_family_name: string_field(
			patient,
			&["patientFamilyName", "patient_family_name"],
		),
		patient_initials_null_flavor: string_field(
			patient,
			&["patientInitialsNullFlavor", "patient_initials_null_flavor"],
		),
		birth_date: None,
		birth_date_null_flavor: string_field(
			patient,
			&["birthDateNullFlavor", "birth_date_null_flavor"],
		),
		age_at_time_of_onset: None,
		age_at_time_of_onset_null_flavor: string_field(
			patient,
			&[
				"ageAtTimeOfOnsetNullFlavor",
				"age_at_time_of_onset_null_flavor",
			],
		),
		age_unit: string_field(patient, &["ageUnit", "age_unit"]),
		gestation_period: None,
		gestation_period_unit: string_field(
			patient,
			&["gestationPeriodUnit", "gestation_period_unit"],
		),
		age_group: string_field(patient, &["ageGroup", "age_group"]),
		weight_kg: None,
		weight_kg_null_flavor: string_field(
			patient,
			&["weightKgNullFlavor", "weight_kg_null_flavor"],
		),
		height_cm: None,
		height_cm_null_flavor: string_field(
			patient,
			&["heightCmNullFlavor", "height_cm_null_flavor"],
		),
		sex: string_field(patient, &["sex"]),
		sex_null_flavor: string_field(
			patient,
			&["sexNullFlavor", "sex_null_flavor"],
		),
		race_code: string_field(patient, &["raceCode", "race_code"]),
		race_code_null_flavor: string_field(
			patient,
			&["raceCodeNullFlavor", "race_code_null_flavor"],
		),
		ethnicity_code: string_field(patient, &["ethnicityCode", "ethnicity_code"]),
		ethnicity_code_null_flavor: string_field(
			patient,
			&["ethnicityCodeNullFlavor", "ethnicity_code_null_flavor"],
		),
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: string_field(
			patient,
			&[
				"lastMenstrualPeriodDateNullFlavor",
				"last_menstrual_period_date_null_flavor",
			],
		),
		medical_history_text: string_field(
			patient,
			&["medicalHistoryText", "medical_history_text"],
		),
		medical_history_text_null_flavor: string_field(
			patient,
			&[
				"medicalHistoryTextNullFlavor",
				"medical_history_text_null_flavor",
			],
		),
		concomitant_therapy: None,
	};
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(_) => {
			PatientInformationBmc::update_by_case(ctx, mm, case_id, update).await?
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {
			PatientInformationBmc::create(
				ctx,
				mm,
				PatientInformationForCreate {
					case_id,
					patient_initials: update.patient_initials,
					patient_given_name: update.patient_given_name,
					patient_family_name: update.patient_family_name,
					patient_initials_null_flavor: update
						.patient_initials_null_flavor,
					birth_date: None,
					birth_date_null_flavor: update.birth_date_null_flavor,
					age_at_time_of_onset: None,
					age_at_time_of_onset_null_flavor: update
						.age_at_time_of_onset_null_flavor,
					age_unit: update.age_unit,
					gestation_period: None,
					gestation_period_unit: update.gestation_period_unit,
					age_group: update.age_group,
					weight_kg: None,
					weight_kg_null_flavor: update.weight_kg_null_flavor,
					height_cm: None,
					height_cm_null_flavor: update.height_cm_null_flavor,
					sex: update.sex,
					sex_null_flavor: update.sex_null_flavor,
					race_code: update.race_code,
					race_code_null_flavor: update.race_code_null_flavor,
					ethnicity_code: update.ethnicity_code,
					ethnicity_code_null_flavor: update.ethnicity_code_null_flavor,
					last_menstrual_period_date: None,
					last_menstrual_period_date_null_flavor: update
						.last_menstrual_period_date_null_flavor,
					medical_history_text: update.medical_history_text,
					medical_history_text_null_flavor: update
						.medical_history_text_null_flavor,
					concomitant_therapy: None,
				},
			)
			.await?;
		}
		Err(err) => return Err(err.into()),
	}
	Ok(())
}

async fn apply_nr_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["narrative", "senderDiagnoses", "caseSummaryInformation"],
	)?;
	let Some(narrative) = optional_row_object(page_id, rows, "narrative")? else {
		return Ok(());
	};
	let case_narrative =
		string_field(narrative, &["caseNarrative", "case_narrative"]);
	let update = NarrativeInformationForUpdate {
		source_narrative_presave_id: uuid_field(
			narrative,
			&["sourceNarrativePresaveId", "source_narrative_presave_id"],
		),
		case_narrative: case_narrative.clone(),
		reporter_comments: string_field(
			narrative,
			&["reporterComments", "reporter_comments"],
		),
		sender_comments: string_field(
			narrative,
			&["senderComments", "sender_comments"],
		),
		additional_information: string_field(
			narrative,
			&["additionalInformation", "additional_information"],
		),
	};
	match NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await? {
		Some(_) => {
			NarrativeInformationBmc::update_by_case(ctx, mm, case_id, update).await?
		}
		None => {
			let Some(case_narrative) = case_narrative else {
				return Ok(());
			};
			NarrativeInformationBmc::create(
				ctx,
				mm,
				NarrativeInformationForCreate {
					case_id,
					source_narrative_presave_id: update.source_narrative_presave_id,
					case_narrative,
					reporter_comments: update.reporter_comments,
					sender_comments: update.sender_comments,
					additional_information: update.additional_information,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn load_editor_rp_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let primary_sources = PrimarySourceBmc::list(
		ctx,
		mm,
		Some(vec![PrimarySourceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!({ "primarySources": primary_sources }))
}

pub async fn get_editor_rp(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PRIMARY_SOURCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_rp_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_rp_page_projection,
	"RP",
	load_editor_rp_data,
	[PRIMARY_SOURCE_LIST],
);

async fn load_editor_sd_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let sender_information = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let sender = sender_information.first().cloned();
	// The SD page patch writes message-header routing fields
	// (messageReceiverIdentifier / batchReceiverIdentifier), so the projection
	// must load the message header back for the edit to round-trip.
	let message_header = match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await
	{
		Ok(entity) => Some(entity),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
		Err(err) => return Err(err.into()),
	};

	Ok(json!({
		"safetyReportIdentification": safety_report_identification,
		"senderInformation": sender_information,
		"sender": sender,
		"messageHeader": message_header,
	}))
}

pub async fn get_editor_sd(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	require_permission(&ctx, SENDER_INFORMATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_sd_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_sd_page_projection,
	"SD",
	load_editor_sd_data,
	[SAFETY_REPORT_READ, SENDER_INFORMATION_LIST],
);

async fn load_editor_lr_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let literature_references = LiteratureReferenceBmc::list(
		ctx,
		mm,
		Some(vec![LiteratureReferenceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!({ "literatureReferences": literature_references }))
}

pub async fn get_editor_lr(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, LITERATURE_REFERENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_lr_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_lr_page_projection,
	"LR",
	load_editor_lr_data,
	[LITERATURE_REFERENCE_LIST],
);

async fn load_editor_si_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let mut studies = StudyInformationBmc::list(
		ctx,
		mm,
		Some(vec![StudyInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	studies.sort_by_key(|study| study.created_at);
	let study_information = studies.into_iter().next();
	let study_registration_numbers = if let Some(ref study) = study_information {
		StudyRegistrationNumberBmc::list(
			ctx,
			mm,
			Some(vec![StudyRegistrationNumberFilter {
				study_information_id: Some(uuid_eq(study.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?
	} else {
		Vec::new()
	};

	Ok(json!({
		"studyInformation": study_information,
		"studyRegistrationNumbers": study_registration_numbers,
	}))
}

pub async fn get_editor_si(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, STUDY_INFORMATION_LIST)?;
	require_permission(&ctx, STUDY_REGISTRATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_si_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_si_page_projection,
	"SI",
	load_editor_si_data,
	[STUDY_INFORMATION_LIST, STUDY_REGISTRATION_LIST],
);

async fn load_editor_dm_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let Some(patient) =
		(match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		})
	else {
		return Ok(json!({
			"patientInformation": null,
			"patientIdentifiers": [],
			"medicalHistoryEpisodes": [],
			"deathInfo": null,
			"reportedCauses": [],
			"autopsyCauses": [],
			"parentInfo": null,
			"parentMedicalHistory": [],
			"parentPastDrugs": [],
		}));
	};

	let patient_id = patient.id;
	let patient_identifiers = PatientIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![PatientIdentifierFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let medical_history_episodes = MedicalHistoryEpisodeBmc::list(
		ctx,
		mm,
		Some(vec![MedicalHistoryEpisodeFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let parent_information_rows = ParentInformationBmc::list(
		ctx,
		mm,
		Some(vec![ParentInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut parents = Vec::new();
	let mut parent_medical_history = Vec::new();
	let mut parent_past_drugs = Vec::new();
	for parent in &parent_information_rows {
		let medical_history = ParentMedicalHistoryBmc::list(
			ctx,
			mm,
			Some(vec![ParentMedicalHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let past_drug_history = ParentPastDrugHistoryBmc::list(
			ctx,
			mm,
			Some(vec![ParentPastDrugHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let mut parent_with_children = json!(parent);
		if let Value::Object(ref mut map) = parent_with_children {
			map.insert("medicalHistory".to_string(), json!(medical_history));
			map.insert("pastDrugHistory".to_string(), json!(past_drug_history));
			map.insert("pastDrugs".to_string(), json!(past_drug_history));
		}
		parent_medical_history.extend(medical_history);
		parent_past_drugs.extend(past_drug_history);
		parents.push(parent_with_children);
	}
	let death_information = PatientDeathInformationBmc::list(
		ctx,
		mm,
		Some(vec![PatientDeathInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut reported_causes = Vec::new();
	let mut autopsy_causes = Vec::new();
	for death_info in &death_information {
		reported_causes.extend(
			ReportedCauseOfDeathBmc::list(
				ctx,
				mm,
				Some(vec![ReportedCauseOfDeathFilter {
					death_info_id: Some(uuid_eq(death_info.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?,
		);
		autopsy_causes.extend(
			AutopsyCauseOfDeathBmc::list(
				ctx,
				mm,
				Some(vec![AutopsyCauseOfDeathFilter {
					death_info_id: Some(uuid_eq(death_info.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?,
		);
	}
	let death_info = death_information.into_iter().next();
	let parent_info = parent_information_rows.into_iter().next();

	Ok(json!({
		"patientInformation": patient,
		"patientIdentifiers": patient_identifiers,
		"medicalHistoryEpisodes": medical_history_episodes,
		"deathInfo": death_info,
		"reportedCauses": reported_causes,
		"autopsyCauses": autopsy_causes,
		"parentInfo": parent_info,
		"parentMedicalHistory": parent_medical_history,
		"parentPastDrugs": parent_past_drugs,
		"parents": parents,
	}))
}

pub async fn get_editor_dm(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PATIENT_READ)?;
	require_permission(&ctx, PATIENT_IDENTIFIER_LIST)?;
	require_permission(&ctx, MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PATIENT_DEATH_LIST)?;
	require_permission(&ctx, DEATH_CAUSE_LIST)?;
	require_permission(&ctx, PARENT_INFORMATION_LIST)?;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PARENT_PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_dm_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_dm_page_projection,
	"DM",
	load_editor_dm_data,
	[
		PATIENT_READ,
		PATIENT_IDENTIFIER_LIST,
		MEDICAL_HISTORY_LIST,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_LIST
	],
);

async fn load_editor_nr_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let narrative =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;
	let (sender_diagnoses, case_summary_information) =
		if let Some(ref narrative) = narrative {
			let sender_diagnoses = SenderDiagnosisBmc::list(
				ctx,
				mm,
				Some(vec![SenderDiagnosisFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			let case_summary_information = CaseSummaryInformationBmc::list(
				ctx,
				mm,
				Some(vec![CaseSummaryInformationFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			(sender_diagnoses, case_summary_information)
		} else {
			(Vec::new(), Vec::new())
		};

	Ok(json!({
		"narrative": narrative,
		"senderDiagnoses": sender_diagnoses,
		"caseSummaryInformation": case_summary_information,
	}))
}

pub async fn get_editor_nr(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, NARRATIVE_READ)?;
	require_permission(&ctx, SENDER_DIAGNOSIS_LIST)?;
	require_permission(&ctx, CASE_SUMMARY_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_nr_data(&ctx, &mm, case_id).await?,
	))
}

direct_page_projection_handler!(
	get_editor_nr_page_projection,
	"NR",
	load_editor_nr_data,
	[NARRATIVE_READ, SENDER_DIAGNOSIS_LIST, CASE_SUMMARY_LIST],
);

#[cfg(test)]
mod tests {
	use super::*;

	fn changes(field: &str, value: Value) -> BTreeMap<String, CaseEditorFieldPatch> {
		let patch = serde_json::from_value(json!({ "value": value }))
			.expect("field patch should deserialize");
		BTreeMap::from([(field.to_string(), patch)])
	}

	#[test]
	fn ci_gate_rejects_invalid_inline_value() {
		let error = validate_direct_changes(
			"CI",
			&changes("reportType", Value::String("9".to_string())),
		)
		.expect_err("invalid report type should fail");
		assert!(format!("{error:?}").contains("ICH.C.1.3.ALLOWED.VALUE"));
	}

	#[test]
	fn ci_gate_validates_null_flavor_values() {
		assert!(validate_direct_changes(
			"CI",
			&changes(
				"fulfilExpeditedCriteriaNullFlavor",
				Value::String("NI".to_string()),
			)
		)
		.is_ok());
		let error = validate_direct_changes(
			"CI",
			&changes(
				"fulfilExpeditedCriteriaNullFlavor",
				Value::String("BAD".to_string()),
			),
		)
		.expect_err("invalid null flavor should fail");
		assert!(format!("{error:?}").contains("ICH.C.1.7.NULLFLAVOR.ALLOWED"));
	}

	#[test]
	fn ci_gate_rejects_non_primitive_patch_values() {
		let error = validate_direct_changes(
			"CI",
			&changes("reportType", json!({ "nested": true })),
		)
		.expect_err("object report type should fail");
		assert!(format!("{error:?}").contains("ICH.C.1.3.LENGTH.MAX"));
	}
}
