use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

pub fn router() -> Router {
	Router::new().merge(
		SwaggerUi::new("/swagger-ui")
			.url("/api-docs/openapi.json", ApiDoc::openapi()),
	)
}

#[derive(OpenApi)]
#[openapi(
	info(
		title = "E2BR3 Backend API",
		version = "0.1.0",
		description = "OpenAPI documentation for the Axum-based E2BR3 backend. Authenticated API routes use the HTTP-only `auth-token` cookie. Internal machine-to-machine routes use the `x-callback-token` header."
	),
	modifiers(&SecurityAddon),
	paths(
		login,
		logoff,
		refresh,
		list_organizations,
		create_organization,
		get_organization,
		update_organization,
		delete_organization,
		list_users,
		create_user,
		get_user,
		update_user,
		delete_user,
		get_current_user,
		set_my_password,
		list_cases,
		create_case,
		check_case_intake_duplicate,
		create_case_from_intake,
		get_case,
		update_case,
		delete_case,
		get_case_patient,
		create_case_patient,
		update_case_patient,
		list_case_reactions,
		create_case_reaction,
		get_case_reaction,
		update_case_reaction,
		delete_case_reaction,
		list_case_drugs,
		create_case_drug,
		get_case_drug,
		update_case_drug,
		delete_case_drug,
		list_case_test_results,
		create_case_test_result,
		get_case_test_result,
		update_case_test_result,
		delete_case_test_result,
		get_case_message_header,
		get_case_receiver,
		get_case_safety_report,
		get_case_narrative,
		list_case_versions,
		validate_case,
		validate_case_all,
		list_case_xml_export_history,
		list_xml_export_history,
		download_xml_export_history_error,
		list_case_link_options,
		export_case_xml,
		get_case_lifecycle,
		submit_case_to_fda,
		submit_case_to_mfds,
		list_case_submissions,
		list_presave_templates,
		create_presave_template,
		get_presave_template,
		update_presave_template,
		delete_presave_template,
		list_presave_template_audits,
		search_meddra,
		search_whodrug,
		import_meddra,
		import_whodrug,
		list_terminology_releases,
		approve_terminology_release,
		activate_terminology_release,
		rollback_terminology_release,
		list_countries,
		get_code_list,
		list_ucum_units,
		list_import_history,
		download_import_history_error,
		validate_import_xml,
		import_xml,
		list_audit_logs,
		verify_audit_log_integrity,
		list_audit_logs_by_record,
		list_validation_rules,
		list_all_submission_history,
		get_case_submission,
		list_submission_event_history,
		download_submission_ack_text,
		get_submission_dispatch_state_view,
		post_mock_ack,
		post_gateway_ack_callback,
		post_reconcile_due_submissions,
		get_reconcile_status
	),
	components(
		schemas(
			LoginRequest,
			LoginResponse,
			LoginResult,
			LogoffRequest,
			LogoffResponse,
			LogoffResult,
			RefreshResponse,
			RefreshData,
			OrganizationDoc,
			OrganizationForCreateDoc,
			OrganizationForUpdateDoc,
			OrganizationResponse,
			OrganizationListResponse,
			CreateOrganizationRequest,
			UpdateOrganizationRequest,
			UserDoc,
			UserForCreateAdminPayloadDoc,
			UserForUpdateDoc,
			UserResponse,
			UserListResponse,
			CreateUserRequest,
			UpdateUserRequest,
			SetMyPasswordBodyDoc,
			SetMyPasswordRequest,
			CaseDoc,
			CaseForCreateDoc,
			CaseForUpdateDoc,
			CaseResponse,
			RawCaseDoc,
			RawCaseResponse,
			CaseListResponse,
			CreateCaseRequest,
			UpdateCaseRequest,
			DeleteCaseRequest,
			CaseIntakeCheckInputDoc,
			CaseIntakeDuplicateMatchDoc,
			CaseIntakeCheckResultDoc,
			CaseIntakeCheckResponse,
			CreateCaseIntakeCheckRequest,
			CaseFromIntakeInputDoc,
			CaseFromIntakeResultDoc,
			CaseFromIntakeResponse,
			CreateCaseFromIntakeRequest,
			PatientInformationDoc,
			PatientInformationForCreateDoc,
			PatientInformationForUpdateDoc,
			PatientInformationResponse,
			CreatePatientInformationRequest,
			UpdatePatientInformationRequest,
			ReactionDoc,
			ReactionForCreateDoc,
			ReactionForUpdateDoc,
			ReactionResponse,
			ReactionListResponse,
			CreateReactionRequest,
			UpdateReactionRequest,
			DrugInformationDoc,
			DrugInformationForCreateDoc,
			DrugInformationForUpdateDoc,
			DrugInformationResponse,
			DrugInformationListResponse,
			CreateDrugInformationRequest,
			UpdateDrugInformationRequest,
			TestResultDoc,
			TestResultForCreateDoc,
			TestResultForUpdateDoc,
			TestResultResponse,
			TestResultListResponse,
			CreateTestResultRequest,
			UpdateTestResultRequest,
			MessageHeaderDoc,
			MessageHeaderForCreateDoc,
			MessageHeaderForUpdateDoc,
			MessageHeaderResponse,
			CreateMessageHeaderRequest,
			UpdateMessageHeaderRequest,
			ReceiverInformationDoc,
			ReceiverInformationForCreateDoc,
			ReceiverInformationForUpdateDoc,
			ReceiverInformationResponse,
			CreateReceiverInformationRequest,
			UpdateReceiverInformationRequest,
			SafetyReportIdentificationDoc,
			SafetyReportIdentificationForCreateDoc,
			SafetyReportIdentificationForUpdateDoc,
			SafetyReportIdentificationResponse,
			CreateSafetyReportIdentificationRequest,
			UpdateSafetyReportIdentificationRequest,
			NarrativeInformationDoc,
			NarrativeInformationForCreateDoc,
			NarrativeInformationForUpdateDoc,
			NarrativeInformationResponse,
			CreateNarrativeInformationRequest,
			UpdateNarrativeInformationRequest,
			ComplianceActionRequest,
			ComplianceActionInputDoc,
			ESignatureInputDoc,
			SubmissionAckDoc,
			SubmissionRecordDoc,
			SubmissionHistoryRecordDoc,
			SubmissionEventRecordDoc,
			SubmissionDispatchStateRecordDoc,
			SubmissionReconcileResultDoc,
			SubmissionReconcileRuntimeStatusDoc,
			MockAckInputDoc,
			GatewayAckCallbackInputDoc,
			ReconcileRequestInputDoc,
			CaseSubmissionListDoc,
			SubmissionEventListDoc,
			SubmissionHistoryListDoc,
			SubmissionDispatchStateDataDoc,
			SubmissionReconcileDataDoc,
			SubmissionReconcileStatusDataDoc,
			SubmissionRecordResponse,
			CaseSubmissionListResponse,
			SubmissionEventListResponse,
			SubmissionHistoryListResponse,
			SubmissionDispatchStateResponse,
			SubmissionReconcileResponse,
			SubmissionReconcileStatusResponse,
			GenericDataRequest,
			GenericDataResponse,
			ErrorResponse
		)
	),
	tags(
		(name = "auth", description = "Authentication and session lifecycle"),
		(name = "organizations", description = "Organization administration"),
		(name = "users", description = "User administration and profile"),
		(name = "cases", description = "Case CRUD operations"),
		(name = "case-subresources", description = "Nested case resources and workflows"),
		(name = "presave-templates", description = "Reusable presave templates"),
		(name = "terminology", description = "Terminology search and release management"),
		(name = "import", description = "XML validation and import"),
		(name = "audit", description = "Audit log APIs"),
		(name = "validation", description = "Validation rule catalog"),
		(name = "submissions", description = "Submission history and ACK processing"),
		(name = "internal", description = "Internal machine-to-machine callbacks")
	)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
	fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
		let components = openapi.components.get_or_insert_with(Default::default);
		components.add_security_scheme(
			"auth_token",
			SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("auth-token"))),
		);
		components.add_security_scheme(
			"callback_token",
			SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(
				"x-callback-token",
			))),
		);
	}
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LoginRequest {
	email: String,
	#[schema(format = Password)]
	pwd: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LoginResult {
	success: bool,
	must_change_password: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LoginResponse {
	result: LoginResult,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LogoffRequest {
	logoff: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LogoffResult {
	logged_off: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct LogoffResponse {
	result: LogoffResult,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct RefreshData {
	expires_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct RefreshResponse {
	data: RefreshData,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ErrorResponse {
	error: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct GenericDataRequest {
	#[schema(value_type = Object)]
	data: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct GenericDataResponse {
	#[schema(value_type = Object)]
	data: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateOrganizationRequest {
	data: OrganizationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateOrganizationRequest {
	data: OrganizationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationResponse {
	data: OrganizationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationListResponse {
	data: Vec<OrganizationDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateUserRequest {
	data: UserForCreateAdminPayloadDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateUserRequest {
	data: UserForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserResponse {
	data: UserDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserListResponse {
	data: Vec<UserDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SetMyPasswordRequest {
	data: SetMyPasswordBodyDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateCaseRequest {
	data: CaseForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateCaseRequest {
	data: CaseForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DeleteCaseRequest {
	reason_for_change: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateCaseIntakeCheckRequest {
	data: CaseIntakeCheckInputDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateCaseFromIntakeRequest {
	data: CaseFromIntakeInputDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseResponse {
	data: CaseDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct RawCaseResponse {
	data: RawCaseDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseListResponse {
	data: Vec<CaseDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseIntakeCheckResponse {
	data: CaseIntakeCheckResultDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseFromIntakeResponse {
	data: CaseFromIntakeResultDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreatePatientInformationRequest {
	data: PatientInformationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdatePatientInformationRequest {
	data: PatientInformationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct PatientInformationResponse {
	data: PatientInformationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateReactionRequest {
	data: ReactionForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateReactionRequest {
	data: ReactionForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReactionResponse {
	data: ReactionDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReactionListResponse {
	data: Vec<ReactionDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateDrugInformationRequest {
	data: DrugInformationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateDrugInformationRequest {
	data: DrugInformationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationResponse {
	data: DrugInformationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationListResponse {
	data: Vec<DrugInformationDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateTestResultRequest {
	data: TestResultForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateTestResultRequest {
	data: TestResultForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct TestResultResponse {
	data: TestResultDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct TestResultListResponse {
	data: Vec<TestResultDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateMessageHeaderRequest {
	data: MessageHeaderForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateMessageHeaderRequest {
	data: MessageHeaderForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct MessageHeaderResponse {
	data: MessageHeaderDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateReceiverInformationRequest {
	data: ReceiverInformationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateReceiverInformationRequest {
	data: ReceiverInformationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReceiverInformationResponse {
	data: ReceiverInformationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateSafetyReportIdentificationRequest {
	data: SafetyReportIdentificationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateSafetyReportIdentificationRequest {
	data: SafetyReportIdentificationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SafetyReportIdentificationResponse {
	data: SafetyReportIdentificationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CreateNarrativeInformationRequest {
	data: NarrativeInformationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateNarrativeInformationRequest {
	data: NarrativeInformationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct NarrativeInformationResponse {
	data: NarrativeInformationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ComplianceActionRequest {
	data: ComplianceActionInputDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionRecordResponse {
	data: SubmissionRecordDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseSubmissionListResponse {
	data: CaseSubmissionListDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionEventListResponse {
	data: SubmissionEventListDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionHistoryListResponse {
	data: SubmissionHistoryListDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionDispatchStateResponse {
	data: SubmissionDispatchStateDataDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileResponse {
	data: SubmissionReconcileDataDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileStatusResponse {
	data: SubmissionReconcileStatusDataDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationDoc {
	id: String,
	name: String,
	#[serde(rename = "type")]
	org_type: Option<String>,
	address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	contact_email: Option<String>,
	contact_phone: Option<String>,
	active: bool,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationForCreateDoc {
	name: String,
	#[serde(rename = "type")]
	org_type: Option<String>,
	address: Option<String>,
	contact_email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationForUpdateDoc {
	name: Option<String>,
	#[serde(rename = "type")]
	org_type: Option<String>,
	address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	contact_email: Option<String>,
	contact_phone: Option<String>,
	active: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserDoc {
	id: String,
	organization_id: String,
	email: String,
	username: String,
	/// Canonical role ID. Built-in values include `system_admin`,
	/// `sponsor_admin_cro`, and `sponsor_admin_company`; other values are
	/// custom scoped roles.
	#[schema(example = "sponsor_admin_cro")]
	role: String,
	first_name: Option<String>,
	last_name: Option<String>,
	comments: Option<String>,
	other_information: Option<String>,
	access_start_at: Option<String>,
	access_end_at: Option<String>,
	access_sender_ids: Option<String>,
	access_product_ids: Option<String>,
	access_study_ids: Option<String>,
	access_blind_allowed: Option<bool>,
	active: bool,
	must_change_password: bool,
	last_login_at: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: Option<String>,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserForCreateAdminPayloadDoc {
	organization_id: String,
	email: String,
	username: Option<String>,
	/// Canonical role ID to assign. Use built-in sponsor admin IDs for fixed
	/// admin roles, or a custom role name for scoped users.
	#[schema(example = "pvs")]
	role: Option<String>,
	first_name: Option<String>,
	last_name: Option<String>,
	comments: Option<String>,
	other_information: Option<String>,
	access_start_at: Option<String>,
	access_end_at: Option<String>,
	access_sender_ids: Option<Vec<String>>,
	access_product_ids: Option<Vec<String>>,
	access_study_ids: Option<Vec<String>>,
	access_blind_allowed: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserForUpdateDoc {
	email: Option<String>,
	username: Option<String>,
	/// Canonical role ID. Legacy values are normalized by the backend to the
	/// new client-aligned role system.
	#[schema(example = "sponsor_admin_company")]
	role: Option<String>,
	first_name: Option<String>,
	last_name: Option<String>,
	comments: Option<String>,
	other_information: Option<String>,
	access_start_at: Option<String>,
	access_end_at: Option<String>,
	access_sender_ids: Option<String>,
	access_product_ids: Option<String>,
	access_study_ids: Option<String>,
	access_blind_allowed: Option<bool>,
	active: Option<bool>,
	last_login_at: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SetMyPasswordBodyDoc {
	#[schema(format = Password)]
	new_password: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseDoc {
	id: String,
	organization_id: String,
	safety_report_id: String,
	version: i32,
	dg_prd_key: Option<String>,
	status: String,
	appendices_json: Option<String>,
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	workflow_status: String,
	workflow_assigned_role: Option<String>,
	workflow_assigned_user_id: Option<String>,
	workflow_due_at: Option<String>,
	workflow_description: Option<String>,
	workflow_updated_at: String,
	qc_state: String,
	is_locked: bool,
	can_act_on_workflow: bool,
	workflow_block_reason: Option<String>,
	mfds_report_type: Option<String>,
	report_year: Option<String>,
	source_document_name: Option<String>,
	source_document_base64: Option<String>,
	source_document_media_type: Option<String>,
	created_by: String,
	updated_by: Option<String>,
	submitted_by: Option<String>,
	submitted_at: Option<String>,
	raw_xml: Option<String>,
	dirty_c: bool,
	dirty_d: bool,
	dirty_e: bool,
	dirty_f: bool,
	dirty_g: bool,
	dirty_h: bool,
	created_at: String,
	updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct RawCaseDoc {
	id: String,
	organization_id: String,
	safety_report_id: String,
	version: i32,
	dg_prd_key: Option<String>,
	status: String,
	appendices_json: Option<String>,
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	workflow_status: String,
	workflow_assigned_role: Option<String>,
	workflow_assigned_user_id: Option<String>,
	workflow_due_at: Option<String>,
	workflow_description: Option<String>,
	workflow_updated_at: String,
	mfds_report_type: Option<String>,
	report_year: Option<String>,
	source_document_name: Option<String>,
	source_document_base64: Option<String>,
	source_document_media_type: Option<String>,
	created_by: String,
	updated_by: Option<String>,
	submitted_by: Option<String>,
	submitted_at: Option<String>,
	raw_xml: Option<String>,
	dirty_c: bool,
	dirty_d: bool,
	dirty_e: bool,
	dirty_f: bool,
	dirty_g: bool,
	dirty_h: bool,
	created_at: String,
	updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseForCreateDoc {
	safety_report_id: String,
	dg_prd_key: Option<String>,
	status: Option<String>,
	appendices_json: Option<String>,
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	workflow_status: String,
	workflow_assigned_role: Option<String>,
	workflow_assigned_user_id: Option<String>,
	workflow_due_at: Option<String>,
	workflow_description: Option<String>,
	workflow_updated_at: String,
	mfds_report_type: Option<String>,
	report_year: Option<String>,
	source_document_name: Option<String>,
	source_document_base64: Option<String>,
	source_document_media_type: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseForUpdateDoc {
	safety_report_id: Option<String>,
	dg_prd_key: Option<String>,
	status: Option<String>,
	appendices_json: Option<String>,
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	mfds_report_type: Option<String>,
	report_year: Option<String>,
	source_document_name: Option<String>,
	source_document_base64: Option<String>,
	source_document_media_type: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseIntakeCheckInputDoc {
	safety_report_id: String,
	#[schema(value_type = Vec<i32>)]
	date_of_most_recent_information: Option<Vec<i32>>,
	report_type: Option<String>,
	reporter_organization: Option<String>,
	sponsor_study_number: Option<String>,
	patient_initials: Option<String>,
	investigation_number: Option<String>,
	age_d2_2a: Option<String>,
	sex_d5: Option<String>,
	dg_prd_key: Option<String>,
	reaction_meddra_version: Option<String>,
	reaction_meddra_code: Option<String>,
	#[schema(value_type = Vec<i32>)]
	ae_start_date: Option<Vec<i32>>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseIntakeDuplicateMatchDoc {
	case_id: String,
	safety_report_id: String,
	version: i32,
	status: String,
	created_at: String,
	report_type: Option<String>,
	#[schema(value_type = Vec<i32>)]
	date_of_most_recent_information: Option<Vec<i32>>,
	reporter_organization: Option<String>,
	sponsor_study_number: Option<String>,
	patient_initials: Option<String>,
	investigation_number: Option<String>,
	age_d2_2a: Option<String>,
	sex_d5: Option<String>,
	dg_prd_key: Option<String>,
	reaction_meddra_version: Option<String>,
	reaction_meddra_code: Option<String>,
	#[schema(value_type = Vec<i32>)]
	ae_start_date: Option<Vec<i32>>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseIntakeCheckResultDoc {
	duplicate: bool,
	/// True when enough duplicate-basis input exists to trust the check.
	/// This is independent of whether warnings are present.
	basis_complete: bool,
	/// Informational messages for duplicate review. This can include
	/// incomplete-basis warnings as well as non-blocking missing-field notes.
	warnings: Vec<String>,
	matches: Vec<CaseIntakeDuplicateMatchDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseFromIntakeInputDoc {
	safety_report_id: String,
	#[schema(value_type = Vec<i32>)]
	transmission_date: Option<Vec<i32>>,
	#[schema(value_type = Vec<i32>)]
	date_first_received_from_source: Option<Vec<i32>>,
	#[schema(value_type = Vec<i32>)]
	date_of_most_recent_information: Vec<i32>,
	report_type: String,
	appendices_json: Option<String>,
	status: Option<String>,
	/// Honored only when duplicate matches are empty and the duplicate basis
	/// is incomplete. Duplicate hits are always hard-blocked.
	allow_duplicate_override: Option<bool>,
	mfds_report_type: Option<String>,
	report_year: Option<String>,
	source_document_name: Option<String>,
	source_document_base64: Option<String>,
	source_document_media_type: Option<String>,
	reporter_organization: Option<String>,
	sponsor_study_number: Option<String>,
	patient_initials: Option<String>,
	investigation_number: Option<String>,
	age_d2_2a: Option<String>,
	sex_d5: Option<String>,
	dg_prd_key: Option<String>,
	reaction_meddra_version: Option<String>,
	reaction_meddra_code: Option<String>,
	#[schema(value_type = Vec<i32>)]
	ae_start_date: Option<Vec<i32>>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseFromIntakeResultDoc {
	case_id: String,
	safety_report_id: String,
	version: i32,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct PatientInformationDoc {
	id: String,
	case_id: String,
	patient_initials: Option<String>,
	patient_given_name: Option<String>,
	patient_family_name: Option<String>,
	birth_date: Option<String>,
	age_at_time_of_onset: Option<String>,
	age_unit: Option<String>,
	gestation_period: Option<String>,
	gestation_period_unit: Option<String>,
	age_group: Option<String>,
	weight_kg: Option<String>,
	height_cm: Option<String>,
	sex: Option<String>,
	patient_initials_null_flavor: Option<String>,
	birth_date_null_flavor: Option<String>,
	age_at_time_of_onset_null_flavor: Option<String>,
	sex_null_flavor: Option<String>,
	race_code: Option<String>,
	ethnicity_code: Option<String>,
	last_menstrual_period_date: Option<String>,
	last_menstrual_period_date_null_flavor: Option<String>,
	medical_history_text: Option<String>,
	concomitant_therapy: Option<bool>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct PatientInformationForCreateDoc {
	case_id: String,
	patient_initials: Option<String>,
	sex: Option<String>,
	concomitant_therapy: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct PatientInformationForUpdateDoc {
	patient_initials: Option<String>,
	patient_given_name: Option<String>,
	patient_family_name: Option<String>,
	patient_initials_null_flavor: Option<String>,
	birth_date: Option<String>,
	birth_date_null_flavor: Option<String>,
	age_at_time_of_onset: Option<String>,
	age_at_time_of_onset_null_flavor: Option<String>,
	age_unit: Option<String>,
	gestation_period: Option<String>,
	gestation_period_unit: Option<String>,
	age_group: Option<String>,
	weight_kg: Option<String>,
	height_cm: Option<String>,
	sex: Option<String>,
	sex_null_flavor: Option<String>,
	race_code: Option<String>,
	ethnicity_code: Option<String>,
	last_menstrual_period_date: Option<String>,
	last_menstrual_period_date_null_flavor: Option<String>,
	medical_history_text: Option<String>,
	concomitant_therapy: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReactionDoc {
	id: String,
	case_id: String,
	sequence_number: i32,
	primary_source_reaction: String,
	primary_source_reaction_translation: Option<String>,
	reaction_language: Option<String>,
	reaction_meddra_version: Option<String>,
	reaction_meddra_code: Option<String>,
	term_highlighted: Option<bool>,
	serious: Option<bool>,
	criteria_death: bool,
	criteria_death_null_flavor: Option<String>,
	criteria_life_threatening: bool,
	criteria_life_threatening_null_flavor: Option<String>,
	criteria_hospitalization: bool,
	criteria_hospitalization_null_flavor: Option<String>,
	criteria_disabling: bool,
	criteria_disabling_null_flavor: Option<String>,
	criteria_congenital_anomaly: bool,
	criteria_congenital_anomaly_null_flavor: Option<String>,
	criteria_other_medically_important: bool,
	criteria_other_medically_important_null_flavor: Option<String>,
	required_intervention: Option<String>,
	start_date: Option<String>,
	start_date_null_flavor: Option<String>,
	end_date: Option<String>,
	end_date_null_flavor: Option<String>,
	duration_value: Option<String>,
	duration_unit: Option<String>,
	outcome: Option<String>,
	medical_confirmation: Option<bool>,
	country_code: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReactionForCreateDoc {
	case_id: String,
	sequence_number: i32,
	primary_source_reaction: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReactionForUpdateDoc {
	primary_source_reaction: Option<String>,
	primary_source_reaction_translation: Option<String>,
	reaction_language: Option<String>,
	reaction_meddra_code: Option<String>,
	reaction_meddra_version: Option<String>,
	term_highlighted: Option<bool>,
	serious: Option<bool>,
	criteria_death: Option<bool>,
	criteria_death_null_flavor: Option<String>,
	criteria_life_threatening: Option<bool>,
	criteria_life_threatening_null_flavor: Option<String>,
	criteria_hospitalization: Option<bool>,
	criteria_hospitalization_null_flavor: Option<String>,
	criteria_disabling: Option<bool>,
	criteria_disabling_null_flavor: Option<String>,
	criteria_congenital_anomaly: Option<bool>,
	criteria_congenital_anomaly_null_flavor: Option<String>,
	criteria_other_medically_important: Option<bool>,
	criteria_other_medically_important_null_flavor: Option<String>,
	required_intervention: Option<String>,
	start_date: Option<String>,
	start_date_null_flavor: Option<String>,
	end_date: Option<String>,
	end_date_null_flavor: Option<String>,
	duration_value: Option<String>,
	duration_unit: Option<String>,
	outcome: Option<String>,
	medical_confirmation: Option<bool>,
	country_code: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationDoc {
	id: String,
	case_id: String,
	sequence_number: i32,
	drug_characterization: String,
	medicinal_product: String,
	mpid: Option<String>,
	mpid_version: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	investigational_product_blinded: Option<bool>,
	obtain_drug_country: Option<String>,
	brand_name: Option<String>,
	drug_generic_name: Option<String>,
	drug_authorization_number: Option<String>,
	manufacturer_name: Option<String>,
	manufacturer_country: Option<String>,
	batch_lot_number: Option<String>,
	cumulative_dose_first_reaction_value: Option<String>,
	cumulative_dose_first_reaction_unit: Option<String>,
	gestation_period_exposure_value: Option<String>,
	gestation_period_exposure_unit: Option<String>,
	dosage_text: Option<String>,
	action_taken: Option<String>,
	rechallenge: Option<String>,
	parent_route: Option<String>,
	parent_route_termid: Option<String>,
	parent_route_termid_version: Option<String>,
	parent_dosage_text: Option<String>,
	fda_additional_info_coded: Option<String>,
	#[schema(value_type = Object)]
	drug_additional_info_codes_json: Option<serde_json::Value>,
	fda_specialized_product_category: Option<String>,
	#[schema(value_type = Object)]
	fda_device_info_json: Option<serde_json::Value>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationForCreateDoc {
	case_id: String,
	sequence_number: i32,
	drug_characterization: String,
	medicinal_product: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationForUpdateDoc {
	medicinal_product: Option<String>,
	drug_characterization: Option<String>,
	brand_name: Option<String>,
	drug_generic_name: Option<String>,
	drug_authorization_number: Option<String>,
	manufacturer_name: Option<String>,
	manufacturer_country: Option<String>,
	batch_lot_number: Option<String>,
	cumulative_dose_first_reaction_value: Option<String>,
	cumulative_dose_first_reaction_unit: Option<String>,
	gestation_period_exposure_value: Option<String>,
	gestation_period_exposure_unit: Option<String>,
	dosage_text: Option<String>,
	action_taken: Option<String>,
	rechallenge: Option<String>,
	investigational_product_blinded: Option<bool>,
	mpid: Option<String>,
	mpid_version: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	obtain_drug_country: Option<String>,
	parent_route: Option<String>,
	parent_route_termid: Option<String>,
	parent_route_termid_version: Option<String>,
	parent_dosage_text: Option<String>,
	fda_additional_info_coded: Option<String>,
	#[schema(value_type = Object)]
	drug_additional_info_codes_json: Option<serde_json::Value>,
	fda_specialized_product_category: Option<String>,
	#[schema(value_type = Object)]
	fda_device_info_json: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct TestResultDoc {
	id: String,
	case_id: String,
	sequence_number: i32,
	test_date: Option<String>,
	test_date_null_flavor: Option<String>,
	test_name: String,
	test_meddra_version: Option<String>,
	test_meddra_code: Option<String>,
	test_result_code: Option<String>,
	test_result_value: Option<String>,
	test_result_unit: Option<String>,
	result_unstructured: Option<String>,
	normal_low_value: Option<String>,
	normal_high_value: Option<String>,
	comments: Option<String>,
	more_info_available: Option<bool>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct TestResultForCreateDoc {
	case_id: String,
	sequence_number: i32,
	test_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct TestResultForUpdateDoc {
	test_name: Option<String>,
	test_date: Option<String>,
	test_date_null_flavor: Option<String>,
	test_meddra_version: Option<String>,
	test_meddra_code: Option<String>,
	test_result_code: Option<String>,
	test_result_value: Option<String>,
	test_result_unit: Option<String>,
	result_unstructured: Option<String>,
	normal_low_value: Option<String>,
	normal_high_value: Option<String>,
	comments: Option<String>,
	more_info_available: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct MessageHeaderDoc {
	id: String,
	case_id: String,
	batch_number: Option<String>,
	batch_sender_identifier: Option<String>,
	batch_receiver_identifier: Option<String>,
	batch_transmission_date: Option<String>,
	message_type: String,
	message_format_version: String,
	message_format_release: String,
	message_number: String,
	message_sender_identifier: String,
	message_receiver_identifier: String,
	message_date_format: String,
	message_date: String,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct MessageHeaderForCreateDoc {
	case_id: String,
	message_number: String,
	message_sender_identifier: String,
	message_receiver_identifier: String,
	message_date: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct MessageHeaderForUpdateDoc {
	batch_number: Option<String>,
	batch_sender_identifier: Option<String>,
	batch_receiver_identifier: Option<String>,
	batch_transmission_date: Option<String>,
	message_number: Option<String>,
	message_sender_identifier: Option<String>,
	message_receiver_identifier: Option<String>,
	message_date: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReceiverInformationDoc {
	id: String,
	case_id: String,
	receiver_type: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state_province: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReceiverInformationForCreateDoc {
	case_id: String,
	receiver_type: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state_province: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReceiverInformationForUpdateDoc {
	receiver_type: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state_province: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SafetyReportIdentificationDoc {
	id: String,
	case_id: String,
	transmission_date: Option<String>,
	transmission_date_null_flavor: Option<String>,
	report_type: String,
	date_first_received_from_source: Option<String>,
	date_first_received_from_source_null_flavor: Option<String>,
	date_of_most_recent_information: Option<String>,
	date_of_most_recent_information_null_flavor: Option<String>,
	fulfil_expedited_criteria: bool,
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	worldwide_unique_id: Option<String>,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
	nullification_code: Option<String>,
	nullification_reason: Option<String>,
	receiver_organization: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SafetyReportIdentificationForCreateDoc {
	case_id: String,
	transmission_date: Option<String>,
	transmission_date_null_flavor: Option<String>,
	report_type: String,
	date_first_received_from_source: Option<String>,
	date_first_received_from_source_null_flavor: Option<String>,
	date_of_most_recent_information: Option<String>,
	date_of_most_recent_information_null_flavor: Option<String>,
	fulfil_expedited_criteria: bool,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SafetyReportIdentificationForUpdateDoc {
	transmission_date: Option<String>,
	transmission_date_null_flavor: Option<String>,
	report_type: Option<String>,
	date_first_received_from_source: Option<String>,
	date_first_received_from_source_null_flavor: Option<String>,
	date_of_most_recent_information: Option<String>,
	date_of_most_recent_information_null_flavor: Option<String>,
	fulfil_expedited_criteria: Option<bool>,
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	worldwide_unique_id: Option<String>,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
	nullification_code: Option<String>,
	nullification_reason: Option<String>,
	receiver_organization: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct NarrativeInformationDoc {
	id: String,
	case_id: String,
	case_narrative: String,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct NarrativeInformationForCreateDoc {
	case_id: String,
	case_narrative: String,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct NarrativeInformationForUpdateDoc {
	case_narrative: Option<String>,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ESignatureInputDoc {
	meaning: String,
	#[schema(format = Password)]
	password: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ComplianceActionInputDoc {
	reason_for_change: String,
	e_signature: ESignatureInputDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionAckDoc {
	level: u8,
	success: bool,
	code: Option<String>,
	message: Option<String>,
	received_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionRecordDoc {
	id: String,
	case_id: String,
	gateway: String,
	remote_submission_id: String,
	status: String,
	xml_bytes: usize,
	submitted_by: String,
	submitted_at: String,
	ack1: Option<SubmissionAckDoc>,
	ack2: Option<SubmissionAckDoc>,
	ack3: Option<SubmissionAckDoc>,
	ack4: Option<SubmissionAckDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionHistoryRecordDoc {
	submission_id: String,
	case_id: String,
	case_number: String,
	gateway: String,
	remote_submission_id: String,
	status: String,
	xml_bytes: usize,
	submitted_by: String,
	submitted_by_email: Option<String>,
	submitted_at: String,
	latest_ack_received_at: Option<String>,
	latest_event_type: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionEventRecordDoc {
	id: String,
	submission_id: String,
	event_type: String,
	#[schema(value_type = Object)]
	event_data: Option<serde_json::Value>,
	created_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionDispatchStateRecordDoc {
	submission_id: String,
	attempt_count: i32,
	last_attempt_at: Option<String>,
	last_error: Option<String>,
	next_retry_at: Option<String>,
	terminal_at: Option<String>,
	created_at: String,
	updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileResultDoc {
	attempted: usize,
	succeeded: usize,
	failed: usize,
	skipped: usize,
	processed_submission_ids: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileRuntimeStatusDoc {
	last_run_at: Option<String>,
	last_success_at: Option<String>,
	last_error: Option<String>,
	total_runs: u64,
	total_errors: u64,
	total_attempted: u64,
	total_succeeded: u64,
	total_failed: u64,
	total_skipped: u64,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct MockAckInputDoc {
	level: u8,
	success: bool,
	code: Option<String>,
	message: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct GatewayAckCallbackInputDoc {
	remote_submission_id: String,
	ack_level: u8,
	success: bool,
	ack_code: Option<String>,
	ack_message: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct ReconcileRequestInputDoc {
	limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseSubmissionListDoc {
	items: Vec<SubmissionRecordDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionEventListDoc {
	items: Vec<SubmissionEventRecordDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionHistoryListDoc {
	items: Vec<SubmissionHistoryRecordDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionDispatchStateDataDoc {
	state: SubmissionDispatchStateRecordDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileDataDoc {
	result: SubmissionReconcileResultDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SubmissionReconcileStatusDataDoc {
	status: SubmissionReconcileRuntimeStatusDoc,
}

#[utoipa::path(
	post,
	path = "/auth/v1/login",
	tag = "auth",
	request_body = LoginRequest,
	responses(
		(status = 200, description = "Authenticated successfully", body = LoginResponse),
		(status = 401, description = "Invalid credentials", body = ErrorResponse)
	)
)]
fn login() {}

#[utoipa::path(
	post,
	path = "/auth/v1/logoff",
	tag = "auth",
	request_body = LogoffRequest,
	responses(
		(status = 200, description = "Session cleared", body = LogoffResponse)
	)
)]
fn logoff() {}

#[utoipa::path(
	post,
	path = "/auth/v1/refresh",
	tag = "auth",
	security(
		("auth_token" = [])
	),
	responses(
		(status = 200, description = "Session refreshed", body = RefreshResponse),
		(status = 401, description = "Authentication required", body = ErrorResponse)
	)
)]
fn refresh() {}

#[utoipa::path(
	get,
	path = "/api/organizations",
	tag = "organizations",
	security(
		("auth_token" = [])
	),
	params(
		("filters" = Option<String>, Query, description = "JSON-encoded filters"),
		("list_options.limit" = Option<i64>, Query, description = "Limit"),
		("list_options.offset" = Option<i64>, Query, description = "Offset"),
		("list_options.order_bys" = Option<String>, Query, description = "Sort order")
	),
	responses(
		(status = 200, description = "Organizations list", body = OrganizationListResponse),
		(status = 403, description = "Admin role required", body = ErrorResponse)
	)
)]
fn list_organizations() {}

#[utoipa::path(
	post,
	path = "/api/organizations",
	tag = "organizations",
	security(
		("auth_token" = [])
	),
	request_body = CreateOrganizationRequest,
	responses(
		(status = 201, description = "Organization created", body = OrganizationResponse),
		(status = 403, description = "Admin role required", body = ErrorResponse)
	)
)]
fn create_organization() {}

#[utoipa::path(
	get,
	path = "/api/organizations/{id}",
	tag = "organizations",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Organization ID")
	),
	responses(
		(status = 200, description = "Organization details", body = OrganizationResponse),
		(status = 404, description = "Organization not found", body = ErrorResponse)
	)
)]
fn get_organization() {}

#[utoipa::path(
	put,
	path = "/api/organizations/{id}",
	tag = "organizations",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Organization ID")
	),
	request_body = UpdateOrganizationRequest,
	responses(
		(status = 200, description = "Organization updated", body = OrganizationResponse),
		(status = 404, description = "Organization not found", body = ErrorResponse)
	)
)]
fn update_organization() {}

#[utoipa::path(
	delete,
	path = "/api/organizations/{id}",
	tag = "organizations",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Organization ID")
	),
	responses(
		(status = 204, description = "Organization deleted"),
		(status = 404, description = "Organization not found", body = ErrorResponse)
	)
)]
fn delete_organization() {}

#[utoipa::path(
	get,
	path = "/api/users",
	tag = "users",
	security(
		("auth_token" = [])
	),
	params(
		("filters" = Option<String>, Query, description = "JSON-encoded filters"),
		("list_options.limit" = Option<i64>, Query, description = "Limit"),
		("list_options.offset" = Option<i64>, Query, description = "Offset"),
		("list_options.order_bys" = Option<String>, Query, description = "Sort order")
	),
	responses(
		(status = 200, description = "Users list", body = UserListResponse),
		(status = 403, description = "Admin role required", body = ErrorResponse)
	)
)]
fn list_users() {}

#[utoipa::path(
	post,
	path = "/api/users",
	tag = "users",
	security(
		("auth_token" = [])
	),
	request_body = CreateUserRequest,
	responses(
		(status = 201, description = "User created", body = UserResponse),
		(status = 403, description = "Admin role required", body = ErrorResponse)
	)
)]
fn create_user() {}

#[utoipa::path(
	get,
	path = "/api/users/{id}",
	tag = "users",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "User ID")
	),
	responses(
		(status = 200, description = "User details", body = UserResponse),
		(status = 404, description = "User not found", body = ErrorResponse)
	)
)]
fn get_user() {}

#[utoipa::path(
	put,
	path = "/api/users/{id}",
	tag = "users",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "User ID")
	),
	request_body = UpdateUserRequest,
	responses(
		(status = 200, description = "User updated", body = UserResponse),
		(status = 404, description = "User not found", body = ErrorResponse)
	)
)]
fn update_user() {}

#[utoipa::path(
	delete,
	path = "/api/users/{id}",
	tag = "users",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "User ID")
	),
	responses(
		(status = 204, description = "User deleted"),
		(status = 404, description = "User not found", body = ErrorResponse)
	)
)]
fn delete_user() {}

#[utoipa::path(
	get,
	path = "/api/users/me",
	tag = "users",
	security(
		("auth_token" = [])
	),
	responses(
		(status = 200, description = "Current user profile", body = UserResponse),
		(status = 401, description = "Authentication required", body = ErrorResponse)
	)
)]
fn get_current_user() {}

#[utoipa::path(
	post,
	path = "/api/users/me/password",
	tag = "users",
	security(
		("auth_token" = [])
	),
	request_body = SetMyPasswordRequest,
	responses(
		(status = 204, description = "Password updated"),
		(status = 400, description = "Invalid password payload", body = ErrorResponse)
	)
)]
fn set_my_password() {}

#[utoipa::path(
	get,
	path = "/api/cases",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	params(
		("filters" = Option<String>, Query, description = "JSON-encoded filters"),
		("list_options.limit" = Option<i64>, Query, description = "Limit"),
		("list_options.offset" = Option<i64>, Query, description = "Offset"),
		("list_options.order_bys" = Option<String>, Query, description = "Sort order")
	),
	responses(
		(status = 200, description = "Cases list", body = CaseListResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse)
	)
)]
fn list_cases() {}

#[utoipa::path(
	post,
	path = "/api/cases",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	request_body = CreateCaseRequest,
	responses(
		(status = 201, description = "Case created", body = CaseResponse),
		(status = 400, description = "Invalid case payload", body = ErrorResponse)
	)
)]
fn create_case() {}

#[utoipa::path(
	post,
	path = "/api/cases/intake-check",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	request_body = CreateCaseIntakeCheckRequest,
	responses(
		(status = 200, description = "Duplicate-check result with basis completeness and warnings", body = CaseIntakeCheckResponse),
		(status = 400, description = "Invalid intake-check payload", body = ErrorResponse)
	)
)]
fn check_case_intake_duplicate() {}

#[utoipa::path(
	post,
	path = "/api/cases/from-intake",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	request_body = CreateCaseFromIntakeRequest,
	responses(
		(status = 201, description = "Case created from intake", body = CaseFromIntakeResponse),
		(status = 400, description = "Duplicate hit hard-blocked, or incomplete basis requires explicit override", body = ErrorResponse)
	)
)]
fn create_case_from_intake() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Case ID")
	),
	responses(
		(status = 200, description = "Case details", body = CaseResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_case() {}

#[utoipa::path(
	put,
	path = "/api/cases/{id}",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Case ID")
	),
	request_body = UpdateCaseRequest,
	responses(
		(status = 200, description = "Case updated", body = CaseResponse),
		(status = 400, description = "Invalid case payload", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn update_case() {}

#[utoipa::path(
	delete,
	path = "/api/cases/{id}",
	tag = "cases",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Case ID")
	),
	request_body = DeleteCaseRequest,
	responses(
		(status = 200, description = "Case soft-deleted", body = RawCaseResponse),
		(status = 400, description = "Invalid delete payload", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn delete_case() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/patient",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Patient for case", body = PatientInformationResponse))
)]
fn get_case_patient() {}

#[utoipa::path(
	post,
	path = "/api/cases/{case_id}/patient",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CreatePatientInformationRequest,
	responses((status = 200, description = "Patient created or updated", body = PatientInformationResponse))
)]
fn create_case_patient() {}

#[utoipa::path(
	put,
	path = "/api/cases/{case_id}/patient",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = UpdatePatientInformationRequest,
	responses((status = 200, description = "Patient updated", body = PatientInformationResponse))
)]
fn update_case_patient() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/reactions",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Reaction collection", body = ReactionListResponse))
)]
fn list_case_reactions() {}

#[utoipa::path(
	post,
	path = "/api/cases/{case_id}/reactions",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CreateReactionRequest,
	responses((status = 201, description = "Reaction created", body = ReactionResponse))
)]
fn create_case_reaction() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/reactions/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Reaction ID")
	),
	responses((status = 200, description = "Reaction item", body = ReactionResponse))
)]
fn get_case_reaction() {}

#[utoipa::path(
	put,
	path = "/api/cases/{case_id}/reactions/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Reaction ID")
	),
	request_body = UpdateReactionRequest,
	responses((status = 200, description = "Reaction updated", body = ReactionResponse))
)]
fn update_case_reaction() {}

#[utoipa::path(
	delete,
	path = "/api/cases/{case_id}/reactions/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Reaction ID")
	),
	responses((status = 204, description = "Reaction deleted"))
)]
fn delete_case_reaction() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/drugs",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Drug collection", body = DrugInformationListResponse))
)]
fn list_case_drugs() {}

#[utoipa::path(
	post,
	path = "/api/cases/{case_id}/drugs",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CreateDrugInformationRequest,
	responses((status = 201, description = "Drug created", body = DrugInformationResponse))
)]
fn create_case_drug() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/drugs/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Drug ID")
	),
	responses((status = 200, description = "Drug item", body = DrugInformationResponse))
)]
fn get_case_drug() {}

#[utoipa::path(
	put,
	path = "/api/cases/{case_id}/drugs/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Drug ID")
	),
	request_body = UpdateDrugInformationRequest,
	responses((status = 200, description = "Drug updated", body = DrugInformationResponse))
)]
fn update_case_drug() {}

#[utoipa::path(
	delete,
	path = "/api/cases/{case_id}/drugs/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Drug ID")
	),
	responses((status = 204, description = "Drug deleted"))
)]
fn delete_case_drug() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/test-results",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Test result collection", body = TestResultListResponse))
)]
fn list_case_test_results() {}

#[utoipa::path(
	post,
	path = "/api/cases/{case_id}/test-results",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CreateTestResultRequest,
	responses((status = 201, description = "Test result created", body = TestResultResponse))
)]
fn create_case_test_result() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/test-results/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Test result ID")
	),
	responses((status = 200, description = "Test result item", body = TestResultResponse))
)]
fn get_case_test_result() {}

#[utoipa::path(
	put,
	path = "/api/cases/{case_id}/test-results/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Test result ID")
	),
	request_body = UpdateTestResultRequest,
	responses((status = 200, description = "Test result updated", body = TestResultResponse))
)]
fn update_case_test_result() {}

#[utoipa::path(
	delete,
	path = "/api/cases/{case_id}/test-results/{id}",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("id" = String, Path, description = "Test result ID")
	),
	responses((status = 204, description = "Test result deleted"))
)]
fn delete_case_test_result() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/message-header",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Message header", body = MessageHeaderResponse))
)]
fn get_case_message_header() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/receiver",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Receiver information", body = ReceiverInformationResponse))
)]
fn get_case_receiver() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/safety-report",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Safety report", body = SafetyReportIdentificationResponse))
)]
fn get_case_safety_report() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/narrative",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Narrative", body = NarrativeInformationResponse))
)]
fn get_case_narrative() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/versions",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Case versions", body = GenericDataResponse))
)]
fn list_case_versions() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/validation",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("profile" = Option<String>, Query, description = "Validation profile override")
	),
	responses((status = 200, description = "Case validation report", body = GenericDataResponse))
)]
fn validate_case() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/validation/all",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "All profile validation report", body = GenericDataResponse))
)]
fn validate_case_all() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}/exports/history",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Case XML export history", body = GenericDataResponse))
)]
fn list_case_xml_export_history() {}

#[utoipa::path(
	get,
	path = "/api/exports/history",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "XML export history", body = GenericDataResponse))
)]
fn list_xml_export_history() {}

#[utoipa::path(
	get,
	path = "/api/exports/history/{id}/error.txt",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Export history record id")),
	responses((status = 200, description = "Export error details text file"))
)]
fn download_xml_export_history_error() {}

#[utoipa::path(
	get,
	path = "/api/cases/link-options",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Case link options", body = GenericDataResponse))
)]
fn list_case_link_options() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}/export/xml",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Case ID"),
		("profile" = Option<String>, Query, description = "Authority-specific appendix export profile: ich, fda, or mfds. Must be selected on the case.")
	),
	responses((status = 200, description = "Case XML export"))
)]
fn export_case_xml() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}/lifecycle",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Case lifecycle", body = GenericDataResponse))
)]
fn get_case_lifecycle() {}

#[utoipa::path(
	post,
	path = "/api/cases/{id}/submissions/fda",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	request_body = ComplianceActionRequest,
	responses((status = 200, description = "FDA submission queued", body = SubmissionRecordResponse))
)]
fn submit_case_to_fda() {}

#[utoipa::path(
	post,
	path = "/api/cases/{id}/submissions/mfds",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	request_body = ComplianceActionRequest,
	responses((status = 200, description = "MFDS submission queued", body = SubmissionRecordResponse))
)]
fn submit_case_to_mfds() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}/submissions",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Submission history for case", body = CaseSubmissionListResponse))
)]
fn list_case_submissions() {}

#[utoipa::path(
	get,
	path = "/api/presave-templates",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	params(("activeOnly" = Option<bool>, Query, description = "Filter active templates only")),
	responses((status = 200, description = "Presave templates", body = GenericDataResponse))
)]
fn list_presave_templates() {}

#[utoipa::path(
	post,
	path = "/api/presave-templates",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	request_body = GenericDataRequest,
	responses((status = 201, description = "Presave template created", body = GenericDataResponse))
)]
fn create_presave_template() {}

#[utoipa::path(
	get,
	path = "/api/presave-templates/{id}",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Template ID")),
	responses((status = 200, description = "Presave template", body = GenericDataResponse))
)]
fn get_presave_template() {}

#[utoipa::path(
	patch,
	path = "/api/presave-templates/{id}",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Template ID")),
	request_body = GenericDataRequest,
	responses((status = 200, description = "Presave template updated", body = GenericDataResponse))
)]
fn update_presave_template() {}

#[utoipa::path(
	delete,
	path = "/api/presave-templates/{id}",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Template ID")),
	responses((status = 204, description = "Presave template deleted"))
)]
fn delete_presave_template() {}

#[utoipa::path(
	get,
	path = "/api/presave-templates/{id}/audit",
	tag = "presave-templates",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Template ID")),
	responses((status = 200, description = "Presave template audit trail", body = GenericDataResponse))
)]
fn list_presave_template_audits() {}

#[utoipa::path(
	get,
	path = "/api/terminology/meddra",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(("query" = Option<String>, Query, description = "Search term")),
	responses((status = 200, description = "MedDRA results", body = GenericDataResponse))
)]
fn search_meddra() {}

#[utoipa::path(
	get,
	path = "/api/terminology/whodrug",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(("query" = Option<String>, Query, description = "Search term")),
	responses((status = 200, description = "WHO Drug results", body = GenericDataResponse))
)]
fn search_whodrug() {}

#[utoipa::path(
	post,
	path = "/api/terminology/import/meddra",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "MedDRA import started", body = GenericDataResponse))
)]
fn import_meddra() {}

#[utoipa::path(
	post,
	path = "/api/terminology/import/whodrug",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "WHO Drug import started", body = GenericDataResponse))
)]
fn import_whodrug() {}

#[utoipa::path(
	get,
	path = "/api/terminology/releases",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Terminology releases", body = GenericDataResponse))
)]
fn list_terminology_releases() {}

#[utoipa::path(
	post,
	path = "/api/terminology/releases/{dictionary}/{version}/approve",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(
		("dictionary" = String, Path, description = "Dictionary key"),
		("version" = String, Path, description = "Release version")
	),
	responses((status = 200, description = "Release approved", body = GenericDataResponse))
)]
fn approve_terminology_release() {}

#[utoipa::path(
	post,
	path = "/api/terminology/releases/{dictionary}/{version}/activate",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(
		("dictionary" = String, Path, description = "Dictionary key"),
		("version" = String, Path, description = "Release version")
	),
	responses((status = 200, description = "Release activated", body = GenericDataResponse))
)]
fn activate_terminology_release() {}

#[utoipa::path(
	post,
	path = "/api/terminology/releases/{dictionary}/{version}/rollback",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(
		("dictionary" = String, Path, description = "Dictionary key"),
		("version" = String, Path, description = "Release version")
	),
	responses((status = 200, description = "Release rolled back", body = GenericDataResponse))
)]
fn rollback_terminology_release() {}

#[utoipa::path(
	get,
	path = "/api/terminology/countries",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Country list", body = GenericDataResponse))
)]
fn list_countries() {}

#[utoipa::path(
	get,
	path = "/api/terminology/code-lists",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(("name" = Option<String>, Query, description = "Code list name")),
	responses((status = 200, description = "Code list entries", body = GenericDataResponse))
)]
fn get_code_list() {}

#[utoipa::path(
	get,
	path = "/api/terminology/ucum-units",
	tag = "terminology",
	security(
		("auth_token" = [])
	),
	params(("query" = Option<String>, Query, description = "Unit search term")),
	responses((status = 200, description = "UCUM units", body = GenericDataResponse))
)]
fn list_ucum_units() {}

#[utoipa::path(
	get,
	path = "/api/import/xml/history",
	tag = "import",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "XML import history", body = GenericDataResponse))
)]
fn list_import_history() {}

#[utoipa::path(
	get,
	path = "/api/import/xml/history/{id}/error.txt",
	tag = "import",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Import history record id")),
	responses((status = 200, description = "Import error details text file"))
)]
fn download_import_history_error() {}

#[utoipa::path(
	post,
	path = "/api/import/xml/validate",
	tag = "import",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "XML validation result", body = GenericDataResponse))
)]
fn validate_import_xml() {}

#[utoipa::path(
	post,
	path = "/api/import/xml",
	tag = "import",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "XML import result", body = GenericDataResponse))
)]
fn import_xml() {}

#[utoipa::path(
	get,
	path = "/api/audit-logs",
	tag = "audit",
	security(
		("auth_token" = [])
	),
	params(
		("filters" = Option<String>, Query, description = "JSON-encoded filters"),
		("list_options.limit" = Option<i64>, Query, description = "Limit"),
		("list_options.offset" = Option<i64>, Query, description = "Offset")
	),
	responses((status = 200, description = "Audit logs", body = GenericDataResponse))
)]
fn list_audit_logs() {}

#[utoipa::path(
	get,
	path = "/api/audit-logs/verify-integrity",
	tag = "audit",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Audit log integrity check", body = GenericDataResponse))
)]
fn verify_audit_log_integrity() {}

#[utoipa::path(
	get,
	path = "/api/audit-logs/by-record/{table_name}/{record_id}",
	tag = "audit",
	security(
		("auth_token" = [])
	),
	params(
		("table_name" = String, Path, description = "Table name"),
		("record_id" = String, Path, description = "Record ID")
	),
	responses((status = 200, description = "Audit logs by record", body = GenericDataResponse))
)]
fn list_audit_logs_by_record() {}

#[utoipa::path(
	get,
	path = "/api/validation/rules",
	tag = "validation",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Validation rule catalog", body = GenericDataResponse))
)]
fn list_validation_rules() {}

#[utoipa::path(
	get,
	path = "/api/submissions/history",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	responses((status = 200, description = "Submission history", body = SubmissionHistoryListResponse))
)]
fn list_all_submission_history() {}

#[utoipa::path(
	get,
	path = "/api/submissions/{id}",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Submission ID")),
	responses((status = 200, description = "Submission details", body = SubmissionRecordResponse))
)]
fn get_case_submission() {}

#[utoipa::path(
	get,
	path = "/api/submissions/{id}/events",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Submission ID")),
	responses((status = 200, description = "Submission event history", body = SubmissionEventListResponse))
)]
fn list_submission_event_history() {}

#[utoipa::path(
	get,
	path = "/api/submissions/{id}/acks/{level}/download",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(
		("id" = String, Path, description = "Submission ID"),
		("level" = u8, Path, description = "ACK level, 1 through 4")
	),
	responses((status = 200, description = "Submission ACK text download", content_type = "text/plain"))
)]
fn download_submission_ack_text() {}

#[utoipa::path(
	get,
	path = "/api/submissions/{id}/dispatch-state",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Submission ID")),
	responses((status = 200, description = "Dispatch state view", body = SubmissionDispatchStateResponse))
)]
fn get_submission_dispatch_state_view() {}

#[utoipa::path(
	post,
	path = "/api/submissions/{id}/acks/mock",
	tag = "submissions",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Submission ID")),
	request_body = MockAckInputDoc,
	responses((status = 200, description = "Mock ACK posted", body = SubmissionRecordResponse))
)]
fn post_mock_ack() {}

#[utoipa::path(
	post,
	path = "/internal/submissions/callbacks/ack",
	tag = "internal",
	security(
		("callback_token" = [])
	),
	request_body = GatewayAckCallbackInputDoc,
	responses((status = 200, description = "Gateway ACK callback accepted", body = SubmissionRecordResponse))
)]
fn post_gateway_ack_callback() {}

#[utoipa::path(
	post,
	path = "/internal/submissions/reconcile",
	tag = "internal",
	security(
		("callback_token" = [])
	),
	request_body = ReconcileRequestInputDoc,
	responses((status = 200, description = "Submission reconciliation triggered", body = SubmissionReconcileResponse))
)]
fn post_reconcile_due_submissions() {}

#[utoipa::path(
	get,
	path = "/internal/submissions/reconcile/status",
	tag = "internal",
	security(
		("callback_token" = [])
	),
	responses((status = 200, description = "Reconciliation status", body = SubmissionReconcileStatusResponse))
)]
fn get_reconcile_status() {}
