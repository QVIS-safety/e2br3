use super::super::common::{
	as_object, bool_field, direct_section_response, explicit_null_model_fields,
	i32_field, json, reject_unknown_row_keys, string_field, uuid_eq, uuid_field,
	BTreeMap, CaseEditorDirectSectionResponse, CtxW, Error, Json, ListOptions,
	ModelManager, Path, PrimarySourceBmc, PrimarySourceFilter,
	PrimarySourceForCreate, PrimarySourceForUpdate, Result, State, Uuid, Value,
};
use super::super::handler_macros::direct_page_projection_handler;
use lib_core::model::safety_report::PrimarySource;

const RP_PRIMARY_SOURCE_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("source_reporter_presave_id", &["sourceReporterPresaveId"]),
	("reporter_title", &["reporterTitle"]),
	("reporter_title_null_flavor", &["reporterTitleNullFlavor"]),
	("reporter_given_name", &["reporterGivenName"]),
	(
		"reporter_given_name_null_flavor",
		&["reporterGivenNameNullFlavor"],
	),
	("reporter_middle_name", &["reporterMiddleName"]),
	(
		"reporter_middle_name_null_flavor",
		&["reporterMiddleNameNullFlavor"],
	),
	("reporter_family_name", &["reporterFamilyName"]),
	(
		"reporter_family_name_null_flavor",
		&["reporterFamilyNameNullFlavor"],
	),
	("organization", &["reporterOrganization"]),
	(
		"organization_null_flavor",
		&["reporterOrganizationNullFlavor"],
	),
	("department", &["reporterDepartment"]),
	("department_null_flavor", &["reporterDepartmentNullFlavor"]),
	("street", &["reporterStreet"]),
	("street_null_flavor", &["reporterStreetNullFlavor"]),
	("city", &["reporterCity"]),
	("city_null_flavor", &["reporterCityNullFlavor"]),
	("state", &["reporterState"]),
	("state_null_flavor", &["reporterStateNullFlavor"]),
	("postcode", &["reporterPostcode"]),
	("postcode_null_flavor", &["reporterPostcodeNullFlavor"]),
	("telephone", &["reporterTelephone"]),
	("telephone_null_flavor", &["reporterTelephoneNullFlavor"]),
	("country_code", &["reporterCountry"]),
	("email", &["reporterEmail"]),
	("email_null_flavor", &["reporterEmailNullFlavor"]),
	("qualification", &["qualification"]),
	("qualification_null_flavor", &["qualificationNullFlavor"]),
	("qualification_kr1", &["qualificationKr1"]),
	(
		"primary_source_regulatory",
		&["primarySourceForRegulatoryPurposes"],
	),
];
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseEditorRpPrimarySourceDto {
	id: Uuid,
	sequence_number: i32,
	reporter_title: Option<String>,
	reporter_title_null_flavor: Option<String>,
	reporter_given_name: Option<String>,
	reporter_given_name_null_flavor: Option<String>,
	reporter_middle_name: Option<String>,
	reporter_middle_name_null_flavor: Option<String>,
	reporter_family_name: Option<String>,
	reporter_family_name_null_flavor: Option<String>,
	#[serde(rename = "reporterOrganization")]
	organization: Option<String>,
	#[serde(rename = "reporterOrganizationNullFlavor")]
	organization_null_flavor: Option<String>,
	#[serde(rename = "reporterDepartment")]
	department: Option<String>,
	#[serde(rename = "reporterDepartmentNullFlavor")]
	department_null_flavor: Option<String>,
	#[serde(rename = "reporterStreet")]
	street: Option<String>,
	#[serde(rename = "reporterStreetNullFlavor")]
	street_null_flavor: Option<String>,
	#[serde(rename = "reporterCity")]
	city: Option<String>,
	#[serde(rename = "reporterCityNullFlavor")]
	city_null_flavor: Option<String>,
	#[serde(rename = "reporterState")]
	state: Option<String>,
	#[serde(rename = "reporterStateNullFlavor")]
	state_null_flavor: Option<String>,
	#[serde(rename = "reporterPostcode")]
	postcode: Option<String>,
	#[serde(rename = "reporterPostcodeNullFlavor")]
	postcode_null_flavor: Option<String>,
	#[serde(rename = "reporterTelephone")]
	telephone: Option<String>,
	#[serde(rename = "reporterTelephoneNullFlavor")]
	telephone_null_flavor: Option<String>,
	#[serde(rename = "reporterCountry")]
	country_code: Option<String>,
	#[serde(rename = "reporterEmail")]
	email: Option<String>,
	#[serde(rename = "reporterEmailNullFlavor")]
	email_null_flavor: Option<String>,
	qualification: Option<String>,
	qualification_null_flavor: Option<String>,
	qualification_kr1: Option<String>,
	#[serde(rename = "primarySourceForRegulatoryPurposes")]
	primary_source_regulatory: Option<String>,
}

impl From<PrimarySource> for CaseEditorRpPrimarySourceDto {
	fn from(source: PrimarySource) -> Self {
		Self {
			id: source.id,
			sequence_number: source.sequence_number,
			reporter_title: source.reporter_title,
			reporter_title_null_flavor: source.reporter_title_null_flavor,
			reporter_given_name: source.reporter_given_name,
			reporter_given_name_null_flavor: source.reporter_given_name_null_flavor,
			reporter_middle_name: source.reporter_middle_name,
			reporter_middle_name_null_flavor: source
				.reporter_middle_name_null_flavor,
			reporter_family_name: source.reporter_family_name,
			reporter_family_name_null_flavor: source
				.reporter_family_name_null_flavor,
			organization: source.organization,
			organization_null_flavor: source.organization_null_flavor,
			department: source.department,
			department_null_flavor: source.department_null_flavor,
			street: source.street,
			street_null_flavor: source.street_null_flavor,
			city: source.city,
			city_null_flavor: source.city_null_flavor,
			state: source.state,
			state_null_flavor: source.state_null_flavor,
			postcode: source.postcode,
			postcode_null_flavor: source.postcode_null_flavor,
			telephone: source.telephone,
			telephone_null_flavor: source.telephone_null_flavor,
			country_code: source.country_code,
			email: source.email,
			email_null_flavor: source.email_null_flavor,
			qualification: source.qualification,
			qualification_null_flavor: source.qualification_null_flavor,
			qualification_kr1: source.qualification_kr1,
			primary_source_regulatory: source.primary_source_regulatory,
		}
	}
}

pub(super) async fn apply_rp_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(page_id, rows, &["primarySources"])?;
	let Some(value) = rows.get("primarySources") else {
		return Ok(());
	};
	let Some(sources) = value.as_array() else {
		return Err(Error::BadRequest {
			message: format!("{page_id}.primarySources must be an array"),
		});
	};
	for value in sources {
		let source = as_object(page_id, "primarySources", value)?;
		apply_rp_source_patch(ctx, mm, case_id, source).await?;
	}
	Ok(())
}

async fn apply_rp_source_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	source: &serde_json::Map<String, Value>,
) -> Result<()> {
	let id = uuid_field(source, &["id"]);
	if bool_field(source, &["deleted"]) == Some(true) {
		if let Some(id) = id {
			PrimarySourceBmc::delete(ctx, mm, id).await?;
		}
		return Ok(());
	}
	let update = PrimarySourceForUpdate {
		source_reporter_presave_id: uuid_field(source, &["sourceReporterPresaveId"]),
		reporter_title: string_field(source, &["reporterTitle"]),
		reporter_title_null_flavor: string_field(
			source,
			&["reporterTitleNullFlavor"],
		),
		reporter_given_name: string_field(source, &["reporterGivenName"]),
		reporter_given_name_null_flavor: string_field(
			source,
			&["reporterGivenNameNullFlavor"],
		),
		reporter_middle_name: string_field(source, &["reporterMiddleName"]),
		reporter_middle_name_null_flavor: string_field(
			source,
			&["reporterMiddleNameNullFlavor"],
		),
		reporter_family_name: string_field(source, &["reporterFamilyName"]),
		reporter_family_name_null_flavor: string_field(
			source,
			&["reporterFamilyNameNullFlavor"],
		),
		organization: string_field(source, &["reporterOrganization"]),
		organization_null_flavor: string_field(
			source,
			&["reporterOrganizationNullFlavor"],
		),
		department: string_field(source, &["reporterDepartment"]),
		department_null_flavor: string_field(
			source,
			&["reporterDepartmentNullFlavor"],
		),
		street: string_field(source, &["reporterStreet"]),
		street_null_flavor: string_field(source, &["reporterStreetNullFlavor"]),
		city: string_field(source, &["reporterCity"]),
		city_null_flavor: string_field(source, &["reporterCityNullFlavor"]),
		state: string_field(source, &["reporterState"]),
		state_null_flavor: string_field(source, &["reporterStateNullFlavor"]),
		postcode: string_field(source, &["reporterPostcode"]),
		postcode_null_flavor: string_field(source, &["reporterPostcodeNullFlavor"]),
		telephone: string_field(source, &["reporterTelephone"]),
		telephone_null_flavor: string_field(
			source,
			&["reporterTelephoneNullFlavor"],
		),
		country_code: string_field(source, &["reporterCountry"]),
		email: string_field(source, &["reporterEmail"]),
		email_null_flavor: string_field(source, &["reporterEmailNullFlavor"]),
		qualification: string_field(source, &["qualification"]),
		qualification_null_flavor: string_field(
			source,
			&["qualificationNullFlavor"],
		),
		qualification_kr1: string_field(source, &["qualificationKr1"]),
		primary_source_regulatory: string_field(
			source,
			&["primarySourceForRegulatoryPurposes"],
		),
	};
	if let Some(id) = id {
		let clear_fields =
			explicit_null_model_fields(source, RP_PRIMARY_SOURCE_PATCH_FIELDS);
		PrimarySourceBmc::update_patch(ctx, mm, id, update, &clear_fields).await?;
	} else {
		PrimarySourceBmc::create(
			ctx,
			mm,
			PrimarySourceForCreate {
				case_id,
				source_reporter_presave_id: update.source_reporter_presave_id,
				sequence_number: i32_field(source, &["sequenceNumber"]).unwrap_or(1),
				reporter_title: update.reporter_title,
				reporter_title_null_flavor: update.reporter_title_null_flavor,
				reporter_given_name: update.reporter_given_name,
				reporter_given_name_null_flavor: update
					.reporter_given_name_null_flavor,
				reporter_middle_name: update.reporter_middle_name,
				reporter_middle_name_null_flavor: update
					.reporter_middle_name_null_flavor,
				reporter_family_name: update.reporter_family_name,
				reporter_family_name_null_flavor: update
					.reporter_family_name_null_flavor,
				organization: update.organization,
				organization_null_flavor: update.organization_null_flavor,
				department: update.department,
				department_null_flavor: update.department_null_flavor,
				street: update.street,
				street_null_flavor: update.street_null_flavor,
				city: update.city,
				city_null_flavor: update.city_null_flavor,
				state: update.state,
				state_null_flavor: update.state_null_flavor,
				postcode: update.postcode,
				postcode_null_flavor: update.postcode_null_flavor,
				telephone: update.telephone,
				telephone_null_flavor: update.telephone_null_flavor,
				country_code: update.country_code,
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

pub(super) async fn load_editor_rp_data(
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
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?
	.into_iter()
	.map(CaseEditorRpPrimarySourceDto::from)
	.collect::<Vec<_>>();

	Ok(json!({ "primarySources": primary_sources }))
}

pub async fn get_editor_rp(
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
		"editor/RP",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_rp_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

direct_page_projection_handler!(
	get_editor_rp_page_projection,
	"RP",
	load_editor_rp_data,
);
