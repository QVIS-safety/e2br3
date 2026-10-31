use super::super::common::{
	direct_section_response, explicit_null_model_fields, json, optional_row_object,
	reject_unknown_row_keys, string_field, uuid_eq, uuid_field, BTreeMap,
	CaseEditorDirectSectionResponse, CtxW, Json, ListOptions, ModelManager, Path,
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate, Result, SenderInformationBmc,
	SenderInformationFilter, SenderInformationForCreate, SenderInformationForUpdate,
	State, Uuid, Value,
};
use super::super::handler_macros::direct_page_projection_handler;

const SD_SENDER_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("source_sender_presave_id", &["sourceSenderPresaveId"]),
	("sender_type", &["senderType"]),
	(
		"health_professional_type_kr1",
		&["healthProfessionalTypeKr1"],
	),
	("organization_name", &["organizationName"]),
	("department", &["department"]),
	("street_address", &["streetAddress"]),
	("city", &["city"]),
	("state", &["state"]),
	("postcode", &["postcode"]),
	("country_code", &["countryCode"]),
	("person_title", &["personTitle"]),
	("person_title_null_flavor", &["personTitleNullFlavor"]),
	("person_given_name", &["personGivenName"]),
	(
		"person_given_name_null_flavor",
		&["personGivenNameNullFlavor"],
	),
	("person_middle_name", &["personMiddleName"]),
	(
		"person_middle_name_null_flavor",
		&["personMiddleNameNullFlavor"],
	),
	("person_family_name", &["personFamilyName"]),
	(
		"person_family_name_null_flavor",
		&["personFamilyNameNullFlavor"],
	),
	("telephone", &["telephone"]),
	("fax", &["fax"]),
	("email", &["email"]),
];
const SD_RECEIVER_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("receiver_type", &["receiverType"]),
	("organization_name", &["organizationName"]),
	("department", &["department"]),
	("street_address", &["streetAddress"]),
	("city", &["city"]),
	("state_province", &["stateProvince"]),
	("postcode", &["postcode"]),
	("country_code", &["countryCode"]),
	("telephone", &["telephone"]),
	("fax", &["fax"]),
	("email", &["email"]),
];
pub(super) async fn apply_sd_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["senderInformation", "receiverInformation"],
	)?;
	if let Some(sender) = optional_row_object(page_id, rows, "senderInformation")? {
		let update = SenderInformationForUpdate {
			source_sender_presave_id: uuid_field(
				sender,
				&["sourceSenderPresaveId"],
			)?,
			sender_type: string_field(sender, &["senderType"]),
			health_professional_type_kr1: string_field(
				sender,
				&["healthProfessionalTypeKr1"],
			),
			organization_name: string_field(sender, &["organizationName"]),
			department: string_field(sender, &["department"]),
			street_address: string_field(sender, &["streetAddress"]),
			city: string_field(sender, &["city"]),
			state: string_field(sender, &["state"]),
			postcode: string_field(sender, &["postcode"]),
			country_code: string_field(sender, &["countryCode"]),
			person_title: string_field(sender, &["personTitle"]),
			person_title_null_flavor: string_field(
				sender,
				&["personTitleNullFlavor"],
			),
			person_given_name: string_field(sender, &["personGivenName"]),
			person_given_name_null_flavor: string_field(
				sender,
				&["personGivenNameNullFlavor"],
			),
			person_middle_name: string_field(sender, &["personMiddleName"]),
			person_middle_name_null_flavor: string_field(
				sender,
				&["personMiddleNameNullFlavor"],
			),
			person_family_name: string_field(sender, &["personFamilyName"]),
			person_family_name_null_flavor: string_field(
				sender,
				&["personFamilyNameNullFlavor"],
			),
			telephone: string_field(sender, &["telephone"]),
			fax: string_field(sender, &["fax"]),
			email: string_field(sender, &["email"]),
		};
		let existing_sender_id = SenderInformationBmc::list(
			ctx,
			mm,
			Some(vec![SenderInformationFilter {
				case_id: Some(uuid_eq(case_id)),
			}]),
			Some(ListOptions::from_limit(1)),
		)
		.await?
		.first()
		.map(|row| row.id);
		if let Some(id) = uuid_field(sender, &["id"])?.or(existing_sender_id) {
			let current = SenderInformationBmc::get(ctx, mm, id).await?;
			if current.case_id != case_id {
				return Err(lib_core::model::Error::EntityUuidNotFound {
					entity: "sender_information",
					id,
				}
				.into());
			}
			let clear_fields =
				explicit_null_model_fields(sender, SD_SENDER_PATCH_FIELDS);
			lib_core::model::update_uuid_patch::<SenderInformationBmc, _>(
				ctx,
				mm,
				id,
				update,
				&clear_fields,
			)
			.await?;
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
					person_title_null_flavor: update.person_title_null_flavor,
					person_given_name: update.person_given_name,
					person_given_name_null_flavor: update
						.person_given_name_null_flavor,
					person_middle_name: update.person_middle_name,
					person_middle_name_null_flavor: update
						.person_middle_name_null_flavor,
					person_family_name: update.person_family_name,
					person_family_name_null_flavor: update
						.person_family_name_null_flavor,
					telephone: update.telephone,
					fax: update.fax,
					email: update.email,
				},
			)
			.await?;
		}
	}
	if let Some(receiver) =
		optional_row_object(page_id, rows, "receiverInformation")?
	{
		let update = ReceiverInformationForUpdate {
			receiver_type: string_field(receiver, &["receiverType"]),
			organization_name: string_field(receiver, &["organizationName"]),
			department: string_field(receiver, &["department"]),
			street_address: string_field(receiver, &["streetAddress"]),
			city: string_field(receiver, &["city"]),
			state_province: string_field(receiver, &["stateProvince"]),
			postcode: string_field(receiver, &["postcode"]),
			country_code: string_field(receiver, &["countryCode"]),
			telephone: string_field(receiver, &["telephone"]),
			fax: string_field(receiver, &["fax"]),
			email: string_field(receiver, &["email"]),
		};
		if ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id)
			.await?
			.is_some()
		{
			let clear_fields =
				explicit_null_model_fields(receiver, SD_RECEIVER_PATCH_FIELDS);
			ReceiverInformationBmc::update_by_case_patch(
				ctx,
				mm,
				case_id,
				update,
				&clear_fields,
			)
			.await?;
		} else {
			ReceiverInformationBmc::create(
				ctx,
				mm,
				ReceiverInformationForCreate {
					case_id,
					receiver_type: update.receiver_type,
					organization_name: update.organization_name,
					department: update.department,
					street_address: update.street_address,
					city: update.city,
					state_province: update.state_province,
					postcode: update.postcode,
					country_code: update.country_code,
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

pub(super) async fn load_editor_sd_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let sender_information = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::from_limit(1)),
	)
	.await?;
	let sender = sender_information.first().cloned();
	let receiver =
		ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;

	Ok(json!({
		"senderInformation": sender.map(|row| json!({
			"id": row.id,
			"sourceSenderPresaveId": row.source_sender_presave_id,
			"senderType": row.sender_type,
			"healthProfessionalTypeKr1": row.health_professional_type_kr1,
			"organizationName": row.organization_name,
			"department": row.department,
			"streetAddress": row.street_address,
			"city": row.city,
			"state": row.state,
			"postcode": row.postcode,
			"countryCode": row.country_code,
			"personTitle": row.person_title,
			"personGivenName": row.person_given_name,
			"personMiddleName": row.person_middle_name,
			"personFamilyName": row.person_family_name,
			"telephone": row.telephone,
			"fax": row.fax,
			"email": row.email,
		})),
		"receiverInformation": receiver.map(|row| json!({
			"id": row.id,
			"receiverType": row.receiver_type,
			"organizationName": row.organization_name,
			"department": row.department,
			"streetAddress": row.street_address,
			"city": row.city,
			"stateProvince": row.state_province,
			"postcode": row.postcode,
			"countryCode": row.country_code,
			"telephone": row.telephone,
			"fax": row.fax,
			"email": row.email,
		})),
	}))
}

pub async fn get_editor_sd(
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
		"editor/SD",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_sd_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

direct_page_projection_handler!(
	get_editor_sd_page_projection,
	"SD",
	load_editor_sd_data,
);
