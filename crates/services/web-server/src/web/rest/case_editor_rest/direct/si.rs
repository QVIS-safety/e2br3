use super::super::common::{
	as_object, bool_field, direct_section_response, explicit_null_model_fields,
	i32_field, json, next_child_sequence, optional_row_object,
	reject_unknown_row_keys, string_field, uuid_eq, uuid_field, BTreeMap,
	CaseEditorDirectSectionResponse, CtxW, Error, Json, ListOptions, ModelManager,
	Path, Result, State, StudyFdaCrossReportedIndBmc,
	StudyFdaCrossReportedIndFilter, StudyFdaCrossReportedIndForCreate,
	StudyFdaCrossReportedIndForUpdate, StudyInformationBmc, StudyInformationFilter,
	StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumberBmc, StudyRegistrationNumberFilter,
	StudyRegistrationNumberForCreate, StudyRegistrationNumberForUpdate, Uuid, Value,
};
use super::super::handler_macros::direct_page_projection_handler;

const SI_STUDY_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("source_study_presave_id", &["sourceStudyPresaveId"]),
	("study_name", &["studyName"]),
	("study_name_null_flavor", &["studyNameNullFlavor"]),
	("sponsor_study_number", &["sponsorStudyNumber"]),
	(
		"sponsor_study_number_null_flavor",
		&["sponsorStudyNumberNullFlavor"],
	),
	("study_type_reaction", &["studyTypeReaction"]),
	("study_type_reaction_kr1", &["studyTypeReactionKr1"]),
	("fda_ind_number_occurred", &["fdaIndNumberOccurred"]),
	(
		"fda_pre_anda_number_occurred",
		&["fdaPreAndaNumberOccurred"],
	),
];
const SI_REGISTRATION_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("registration_number", &["registrationNumber"]),
	(
		"registration_number_null_flavor",
		&["registrationNumberNullFlavor"],
	),
	("country_code", &["countryCode"]),
	("country_code_null_flavor", &["countryCodeNullFlavor"]),
];
const SI_FDA_IND_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("ind_number", &["indNumber"]),
	("ind_number_null_flavor", &["indNumberNullFlavor"]),
];
pub(super) async fn apply_si_page_rows_patch(
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
	let study = optional_row_object(page_id, rows, "studyInformation")?;
	let study_id = if let Some(study) = study {
		let update = StudyInformationForUpdate {
			source_study_presave_id: uuid_field(study, &["sourceStudyPresaveId"]),
			study_name: string_field(study, &["studyName"]),
			study_name_null_flavor: string_field(study, &["studyNameNullFlavor"]),
			sponsor_study_number: string_field(study, &["sponsorStudyNumber"]),
			sponsor_study_number_null_flavor: string_field(
				study,
				&["sponsorStudyNumberNullFlavor"],
			),
			study_type_reaction: string_field(study, &["studyTypeReaction"]),
			study_type_reaction_kr1: string_field(study, &["studyTypeReactionKr1"]),
			fda_ind_number_occurred: string_field(study, &["fdaIndNumberOccurred"]),
			fda_pre_anda_number_occurred: string_field(
				study,
				&["fdaPreAndaNumberOccurred"],
			),
		};
		if let Some(id) = uuid_field(study, &["id"]) {
			let clear_fields =
				explicit_null_model_fields(study, SI_STUDY_PATCH_FIELDS);
			lib_core::model::update_uuid_patch::<StudyInformationBmc, _>(
				ctx,
				mm,
				id,
				update,
				&clear_fields,
			)
			.await?;
			id
		} else {
			let existing = StudyInformationBmc::list(
				ctx,
				mm,
				Some(vec![StudyInformationFilter {
					case_id: Some(uuid_eq(case_id)),
				}]),
				Some(ListOptions::from_order_bys(vec!["created_at", "id"])),
			)
			.await?
			.into_iter()
			.min_by_key(|study| study.created_at);
			if let Some(existing) = existing {
				let clear_fields =
					explicit_null_model_fields(study, SI_STUDY_PATCH_FIELDS);
				lib_core::model::update_uuid_patch::<StudyInformationBmc, _>(
					ctx,
					mm,
					existing.id,
					update,
					&clear_fields,
				)
				.await?;
				existing.id
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
						fda_pre_anda_number_occurred: update
							.fda_pre_anda_number_occurred,
					},
				)
				.await?
			}
		}
	} else {
		let studies = StudyInformationBmc::list(
			ctx,
			mm,
			Some(vec![StudyInformationFilter {
				case_id: Some(uuid_eq(case_id)),
			}]),
			Some(ListOptions::from_order_bys(vec!["created_at", "id"])),
		)
		.await?;
		let Some(study) = studies.into_iter().min_by_key(|study| study.created_at)
		else {
			if rows.contains_key("studyRegistrationNumbers") {
				return Err(Error::BadRequest {
					message: format!(
						"{page_id}.studyInformation is required before child rows"
					),
				});
			}
			return Ok(());
		};
		study.id
	};

	if let Some(value) = rows.get("studyRegistrationNumbers") {
		let Some(registrations) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!(
					"{page_id}.studyRegistrationNumbers must be an array"
				),
			});
		};
		for value in registrations {
			let registration =
				as_object(page_id, "studyRegistrationNumbers", value)?;
			let id = uuid_field(registration, &["id"]);
			if bool_field(registration, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					let existing =
						StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
					if existing.study_information_id != study_id {
						return Err(lib_core::model::Error::EntityUuidNotFound {
							entity: "study_registration_numbers",
							id,
						}
						.into());
					}
					StudyRegistrationNumberBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let update = StudyRegistrationNumberForUpdate {
				registration_number: string_field(
					registration,
					&["registrationNumber"],
				),
				registration_number_null_flavor: string_field(
					registration,
					&["registrationNumberNullFlavor"],
				),
				country_code: string_field(registration, &["countryCode"]),
				country_code_null_flavor: string_field(
					registration,
					&["countryCodeNullFlavor"],
				),
				sequence_number: i32_field(registration, &["sequenceNumber"]),
			};
			if let Some(id) = id {
				let existing = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				if existing.study_information_id != study_id {
					return Err(lib_core::model::Error::EntityUuidNotFound {
						entity: "study_registration_numbers",
						id,
					}
					.into());
				}
				let clear_fields = explicit_null_model_fields(
					registration,
					SI_REGISTRATION_PATCH_FIELDS,
				);
				lib_core::model::update_uuid_patch::<StudyRegistrationNumberBmc, _>(
					ctx,
					mm,
					id,
					update,
					&clear_fields,
				)
				.await?;
			} else if update.registration_number.is_some()
				|| update.registration_number_null_flavor.is_some()
			{
				StudyRegistrationNumberBmc::create(
					ctx,
					mm,
					StudyRegistrationNumberForCreate {
						study_information_id: study_id,
						registration_number: update.registration_number,
						registration_number_null_flavor: update
							.registration_number_null_flavor,
						country_code: update.country_code,
						country_code_null_flavor: update.country_code_null_flavor,
						sequence_number: update.sequence_number.unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"study_registration_numbers",
								"study_information_id",
								study_id,
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

	if let Some(study) = study {
		if let Some(value) = study.get("fdaCrossReportedIndNumbers") {
			let Some(numbers) = value.as_array() else {
				return Err(Error::BadRequest {
					message: format!(
						"{page_id}.studyInformation.fdaCrossReportedIndNumbers must be an array"
					),
				});
			};
			for value in numbers {
				let number = as_object(
					page_id,
					"studyInformation.fdaCrossReportedIndNumbers",
					value,
				)?;
				let id = uuid_field(number, &["id"]);
				if bool_field(number, &["deleted"]) == Some(true) {
					if let Some(id) = id {
						let existing =
							StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
						if existing.study_information_id != study_id {
							return Err(
								lib_core::model::Error::EntityUuidNotFound {
									entity: "study_fda_cross_reported_inds",
									id,
								}
								.into(),
							);
						}
						StudyFdaCrossReportedIndBmc::delete(ctx, mm, id).await?;
					}
					continue;
				}
				let update = StudyFdaCrossReportedIndForUpdate {
					ind_number: string_field(number, &["indNumber"]),
					ind_number_null_flavor: string_field(
						number,
						&["indNumberNullFlavor"],
					),
					sequence_number: i32_field(number, &["sequenceNumber"]),
				};
				if let Some(id) = id {
					let existing =
						StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
					if existing.study_information_id != study_id {
						return Err(lib_core::model::Error::EntityUuidNotFound {
							entity: "study_fda_cross_reported_inds",
							id,
						}
						.into());
					}
					let clear_fields =
						explicit_null_model_fields(number, SI_FDA_IND_PATCH_FIELDS);
					lib_core::model::update_uuid_patch::<
						StudyFdaCrossReportedIndBmc,
						_,
					>(ctx, mm, id, update, &clear_fields)
					.await?;
				} else {
					StudyFdaCrossReportedIndBmc::create(
						ctx,
						mm,
						StudyFdaCrossReportedIndForCreate {
							study_information_id: study_id,
							ind_number: update.ind_number,
							ind_number_null_flavor: update.ind_number_null_flavor,
							sequence_number: update.sequence_number.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"study_fda_cross_reported_inds",
									"study_information_id",
									study_id,
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
	}
	Ok(())
}

pub(super) async fn load_editor_si_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let mut study_options = ListOptions::from_order_bys(vec!["created_at", "id"]);
	study_options.limit = Some(1);
	let study_information = StudyInformationBmc::list(
		ctx,
		mm,
		Some(vec![StudyInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(study_options),
	)
	.await?
	.into_iter()
	.next();
	let (study_registration_numbers, fda_cross_reported_ind_numbers) =
		if let Some(ref study) = study_information {
			let registrations = StudyRegistrationNumberBmc::list(
				ctx,
				mm,
				Some(vec![StudyRegistrationNumberFilter {
					study_information_id: Some(uuid_eq(study.id)),
					..Default::default()
				}]),
				Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
			)
			.await?;
			let cross_reported = StudyFdaCrossReportedIndBmc::list(
				ctx,
				mm,
				Some(vec![StudyFdaCrossReportedIndFilter {
					study_information_id: Some(uuid_eq(study.id)),
					..Default::default()
				}]),
				Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
			)
			.await?;
			(registrations, cross_reported)
		} else {
			(Vec::new(), Vec::new())
		};
	let study_information = study_information.map(|study| {
		let mut value = json!(study);
		value
			.as_object_mut()
			.expect("serialized study information is an object")
			.insert(
				"fdaCrossReportedIndNumbers".to_string(),
				json!(fda_cross_reported_ind_numbers),
			);
		value
	});

	Ok(json!({
		"studyInformation": study_information,
		"studyRegistrationNumbers": study_registration_numbers,
	}))
}

pub async fn get_editor_si(
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
		"editor/SI",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_si_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

direct_page_projection_handler!(
	get_editor_si_page_projection,
	"SI",
	load_editor_si_data,
);
