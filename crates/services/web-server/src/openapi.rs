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
		title = "QVIS Safety Backend API",
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
		get_current_user_profile,
		set_my_password,
		list_cases,
		create_case,
		check_case_intake_duplicate,
		create_case_from_intake,
		get_case,
		update_case,
		delete_case,
		get_editor_shell,
		get_editor_ci,
		get_editor_ci_page,
		patch_editor_ci_page,
		get_editor_rp_page,
		patch_editor_rp_page,
		get_editor_sd_page,
		patch_editor_sd_page,
		get_editor_lr_page,
		patch_editor_lr_page,
		get_editor_si_page,
		patch_editor_si_page,
		get_editor_dm_page,
		patch_editor_dm_page,
		get_editor_nr_page,
		patch_editor_nr_page,
		get_editor_dh_page,
		get_editor_ae_page,
		get_editor_lb_page,
		get_editor_dg_page,
		get_editor_dh_page_row,
		get_editor_ae_page_row,
		get_editor_lb_page_row,
		get_editor_dg_page_row,
		create_editor_repeatable_page_row,
		patch_editor_repeatable_page_row,
		delete_editor_repeatable_page_row,
		get_editor_rp,
		get_editor_sd,
		get_editor_lr,
		get_editor_si,
		get_editor_dm,
		get_editor_nr,
		list_editor_dh,
		get_editor_dh,
		list_editor_ae,
		get_editor_ae,
		list_editor_lb,
		get_editor_lb,
		list_editor_dg,
		get_editor_dg,
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
		preview_case_narrative,
		list_case_versions,
		validate_case,
		list_case_xml_export_history,
		list_xml_export_history,
		download_xml_export_history_error,
		list_case_link_options,
		export_case_xml,
		export_case_cioms_pdf,
		get_case_lifecycle,
		submit_case_to_fda,
		submit_case_to_mfds,
		list_case_submissions,
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
			CurrentUserProfileResponse,
			CurrentUserProfileDoc,
			RoutingProfileDoc,
			RoutingSenderOptionDoc,
			EffectiveScopeSummaryDoc,
			UserCapabilitiesDoc,
			ModuleCrudCapabilitiesDoc,
			CaseCapabilitiesDoc,
			ExecuteCapabilitiesDoc,
			DataCapabilitiesDoc,
			AdminCapabilitiesDoc,
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
			CaseEditorShellDoc,
			CaseEditorDirectSectionResponseDoc,
			CaseEditorPageProjectionResponseDoc,
			CaseEditorPagePatchRequestDoc,
			CaseEditorFieldPatchDoc,
			CaseEditorFieldEnvelopeDoc,
			CaseEditorFieldIssueDoc,
			CaseEditorRowDetailResponseDoc,
			CaseEditorAeListRowDoc,
			CaseEditorAeListResponseDoc,
			CaseEditorLbListRowDoc,
			CaseEditorLbListResponseDoc,
			CaseEditorDgListRowDoc,
			CaseEditorDgListResponseDoc,
			CaseEditorDhListRowDoc,
			CaseEditorDhListResponseDoc,
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
			SenderInformationDoc,
			SenderInformationForCreateDoc,
			SenderInformationForUpdateDoc,
			SenderInformationResponse,
			SenderInformationListResponse,
			CreateSenderInformationRequest,
			UpdateSenderInformationRequest,
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
		(name = "case-editor", description = "Explicit case editor read APIs"),
		(name = "case-subresources", description = "Nested case resources and workflows"),
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
struct CurrentUserProfileResponse {
	data: CurrentUserProfileDoc,
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
#[serde(rename_all = "camelCase")]
struct CaseEditorShellDoc {
	id: String,
	status: String,
	organization_id: String,
	safety_report_id: String,
	dg_prd_key: Option<String>,
	created_at: String,
	updated_at: String,
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
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorDirectSectionResponseDoc {
	case_id: String,
	#[schema(value_type = Object)]
	data: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorPageProjectionResponseDoc {
	case_id: String,
	page_id: String,
	authorities: Vec<String>,
	saved: bool,
	required_count: usize,
	#[schema(value_type = Object)]
	fields: serde_json::Value,
	#[schema(value_type = Object)]
	rows: serde_json::Value,
	section_summaries: Vec<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorPagePatchRequestDoc {
	/// Validation/render authorities for page projection: ich,fda,mfds.
	authorities: Option<Vec<String>>,
	#[schema(value_type = Object)]
	changes: serde_json::Value,
	#[schema(value_type = Object)]
	rows: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorFieldPatchDoc {
	#[schema(value_type = Object)]
	value: serde_json::Value,
	null_flavor: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorFieldEnvelopeDoc {
	field_id: String,
	path: String,
	label: String,
	#[schema(value_type = Object)]
	value: serde_json::Value,
	display: Option<String>,
	null_flavor: Option<String>,
	notation: Option<String>,
	#[schema(value_type = Object)]
	origin_value: serde_json::Value,
	origin_null_flavor: Option<String>,
	visible: bool,
	editable: bool,
	empty: bool,
	required_empty: bool,
	issues: Vec<CaseEditorFieldIssueDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorFieldIssueDoc {
	code: String,
	message: String,
	blocking: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorRowDetailResponseDoc {
	case_id: String,
	section: Option<String>,
	row_id: String,
	authorities: Vec<String>,
	#[schema(value_type = Object)]
	data: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorAeListRowDoc {
	id: String,
	sequence_number: i32,
	reaction_primary_source_native: String,
	reaction_primary_source_translation: Option<String>,
	meddra_version: Option<String>,
	meddra_code: Option<String>,
	seriousness: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorAeListResponseDoc {
	case_id: String,
	rows: Vec<CaseEditorAeListRowDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorLbListRowDoc {
	id: String,
	sequence_number: i32,
	test_name: String,
	test_date: Option<String>,
	result_value: Option<String>,
	result_unit: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorLbListResponseDoc {
	case_id: String,
	rows: Vec<CaseEditorLbListRowDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorDgListRowDoc {
	id: String,
	sequence_number: i32,
	drug_role: String,
	dg_prd_key: Option<String>,
	medicinal_product: String,
	action_taken: Option<String>,
	warning_count: i32,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorDgListResponseDoc {
	case_id: String,
	rows: Vec<CaseEditorDgListRowDoc>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorDhListRowDoc {
	id: String,
	sequence_number: i32,
	drug_name: Option<String>,
	indication: Option<String>,
	start_date: Option<String>,
	end_date: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseEditorDhListResponseDoc {
	case_id: String,
	rows: Vec<CaseEditorDhListRowDoc>,
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
struct CreateSenderInformationRequest {
	data: SenderInformationForCreateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UpdateSenderInformationRequest {
	data: SenderInformationForUpdateDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SenderInformationResponse {
	data: SenderInformationDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SenderInformationListResponse {
	data: Vec<SenderInformationDoc>,
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
	#[schema(example = "cro")]
	#[serde(rename = "type", alias = "org_type")]
	org_type: Option<String>,
	address: Option<String>,
	contact_email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct OrganizationForUpdateDoc {
	name: Option<String>,
	#[schema(example = "pharmaceutical_company")]
	#[serde(rename = "type", alias = "org_type")]
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
#[serde(rename_all = "camelCase")]
struct UserRoleMetadataDoc {
	canonical_role_id: String,
	display_name: String,
	is_builtin: bool,
	is_editable: bool,
	is_sponsor_admin: bool,
	is_operational: bool,
	can_admin: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct UserScopeDoc {
	assigned_sender_ids: Vec<String>,
	assigned_product_ids: Vec<String>,
	assigned_study_ids: Vec<String>,
	access_blind_allowed: bool,
	active_sender_identifier: Option<String>,
	access_start_at: Option<String>,
	access_end_at: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
	role_meta: UserRoleMetadataDoc,
	comments: Option<String>,
	other_information: Option<String>,
	scope: UserScopeDoc,
	active: bool,
	must_change_password: bool,
	last_login_at: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: Option<String>,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct RoutingSenderOptionDoc {
	sender_identifier: String,
	case_count: i64,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct EffectiveScopeSummaryDoc {
	assigned_sender_ids: Vec<String>,
	assigned_product_ids: Vec<String>,
	assigned_study_ids: Vec<String>,
	access_blind_allowed: bool,
	active_sender_identifier: Option<String>,
	effective_sender_filter: Option<String>,
	access_start_at: Option<String>,
	access_end_at: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct RoutingProfileDoc {
	built_in_role_id: String,
	operational: bool,
	sender_selection_required: bool,
	active_sender_identifier: Option<String>,
	available_senders: Vec<RoutingSenderOptionDoc>,
	effective_scope: EffectiveScopeSummaryDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ModuleCrudCapabilitiesDoc {
	/// Permission to view records in this module.
	read: bool,
	/// Permission to create records in this module.
	create: bool,
	/// Permission to update records in this module.
	update: bool,
	/// Permission to delete records in this module.
	delete: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CaseCapabilitiesDoc {
	read: bool,
	create: bool,
	update: bool,
	delete: bool,
	/// Permission to approve or review cases.
	review: bool,
	/// Permission to lock cases. Currently follows case approval permission.
	lock: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ExecuteCapabilitiesDoc {
	/// Permission to view history or metadata for this operation.
	read: bool,
	/// Permission to run this operation.
	execute: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct DataCapabilitiesDoc {
	read: bool,
	/// Permission to import terminology data.
	import: bool,
	/// Permission to approve terminology data.
	approve: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct AdminCapabilitiesDoc {
	/// Permission to open admin surfaces.
	read: bool,
	/// Permission to make admin-level changes.
	update: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct UserCapabilitiesDoc {
	case: CaseCapabilitiesDoc,
	info: ModuleCrudCapabilitiesDoc,
	import: ExecuteCapabilitiesDoc,
	export_submission: ExecuteCapabilitiesDoc,
	data: DataCapabilitiesDoc,
	admin: AdminCapabilitiesDoc,
	users: ModuleCrudCapabilitiesDoc,
	roles: ModuleCrudCapabilitiesDoc,
	settings: AdminCapabilitiesDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CurrentUserProfileDoc {
	user: UserDoc,
	routing: RoutingProfileDoc,
	/// Backend-derived permissions for the authenticated user's UI affordances.
	capabilities: UserCapabilitiesDoc,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct UserForCreateAdminPayloadDoc {
	organization_id: String,
	email: String,
	username: Option<String>,
	/// Optional initial password. Defaults to "welcome" when omitted.
	#[schema(format = Password)]
	pwd_clear: Option<String>,
	/// Canonical role ID to assign. Use built-in sponsor admin IDs for fixed
	/// admin roles, or a custom role name for scoped users.
	#[schema(example = "user")]
	role: Option<String>,
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
	fda_report_type: Option<String>,
	report_year: Option<String>,
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
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	workflow_status: String,
	workflow_assigned_role: Option<String>,
	workflow_assigned_user_id: Option<String>,
	workflow_due_at: Option<String>,
	workflow_description: Option<String>,
	workflow_updated_at: String,
	mfds_report_type: Option<String>,
	fda_report_type: Option<String>,
	report_year: Option<String>,
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
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	workflow_status: String,
	workflow_assigned_role: Option<String>,
	workflow_assigned_user_id: Option<String>,
	workflow_due_at: Option<String>,
	workflow_description: Option<String>,
	workflow_updated_at: String,
	mfds_report_type: Option<String>,
	fda_report_type: Option<String>,
	report_year: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct CaseForUpdateDoc {
	safety_report_id: Option<String>,
	dg_prd_key: Option<String>,
	status: Option<String>,
	review_receivers_json: Option<String>,
	workflow_routes_json: Option<String>,
	mfds_report_type: Option<String>,
	fda_report_type: Option<String>,
	report_year: Option<String>,
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
	status: Option<String>,
	/// Honored only when duplicate matches are empty and the duplicate basis
	/// is incomplete. Duplicate hits are always hard-blocked.
	allow_duplicate_override: Option<bool>,
	mfds_report_type: Option<String>,
	fda_report_type: Option<String>,
	report_year: Option<String>,
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
	included_in_ema_ime_list: Option<bool>,
	expectedness: Option<String>,
	severity: Option<String>,
	mfds_device_ae_classification: Option<String>,
	mfds_device_ae_outcome: Option<String>,
	mfds_device_cause_medical_device: Option<bool>,
	mfds_device_cause_procedure_issue: Option<bool>,
	mfds_device_cause_patient_condition: Option<bool>,
	mfds_device_cause_unable_to_assess: Option<bool>,
	mfds_device_cause_other: Option<String>,
	mfds_device_action_reason: Option<String>,
	mfds_device_action_recall: Option<bool>,
	mfds_device_action_repair: Option<bool>,
	mfds_device_action_inspection: Option<bool>,
	mfds_device_action_replacement: Option<bool>,
	mfds_device_action_improvement: Option<bool>,
	mfds_device_action_monitoring: Option<bool>,
	mfds_device_action_notification: Option<bool>,
	mfds_device_action_label_change: Option<bool>,
	mfds_device_action_other: Option<String>,
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
	included_in_ema_ime_list: Option<bool>,
	expectedness: Option<String>,
	severity: Option<String>,
	mfds_device_ae_classification: Option<String>,
	mfds_device_ae_outcome: Option<String>,
	mfds_device_cause_medical_device: Option<bool>,
	mfds_device_cause_procedure_issue: Option<bool>,
	mfds_device_cause_patient_condition: Option<bool>,
	mfds_device_cause_unable_to_assess: Option<bool>,
	mfds_device_cause_other: Option<String>,
	mfds_device_action_reason: Option<String>,
	mfds_device_action_recall: Option<bool>,
	mfds_device_action_repair: Option<bool>,
	mfds_device_action_inspection: Option<bool>,
	mfds_device_action_replacement: Option<bool>,
	mfds_device_action_improvement: Option<bool>,
	mfds_device_action_monitoring: Option<bool>,
	mfds_device_action_notification: Option<bool>,
	mfds_device_action_label_change: Option<bool>,
	mfds_device_action_other: Option<String>,
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
	included_in_ema_ime_list: Option<bool>,
	expectedness: Option<String>,
	severity: Option<String>,
	mfds_device_ae_classification: Option<String>,
	mfds_device_ae_outcome: Option<String>,
	mfds_device_cause_medical_device: Option<bool>,
	mfds_device_cause_procedure_issue: Option<bool>,
	mfds_device_cause_patient_condition: Option<bool>,
	mfds_device_cause_unable_to_assess: Option<bool>,
	mfds_device_cause_other: Option<String>,
	mfds_device_action_reason: Option<String>,
	mfds_device_action_recall: Option<bool>,
	mfds_device_action_repair: Option<bool>,
	mfds_device_action_inspection: Option<bool>,
	mfds_device_action_replacement: Option<bool>,
	mfds_device_action_improvement: Option<bool>,
	mfds_device_action_monitoring: Option<bool>,
	mfds_device_action_notification: Option<bool>,
	mfds_device_action_label_change: Option<bool>,
	mfds_device_action_other: Option<String>,
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
	source_product_presave_id: Option<String>,
	sequence_number: i32,
	drug_characterization: String,
	medicinal_product: String,
	mpid: Option<String>,
	mpid_version: Option<String>,
	mfds_mpid_version: Option<String>,
	mfds_mpid: Option<String>,
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
	source_product_presave_id: Option<String>,
	sequence_number: i32,
	drug_characterization: String,
	medicinal_product: String,
	mpid: Option<String>,
	mpid_version: Option<String>,
	mfds_mpid_version: Option<String>,
	mfds_mpid: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct DrugInformationForUpdateDoc {
	source_product_presave_id: Option<String>,
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
	mfds_mpid_version: Option<String>,
	mfds_mpid: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	obtain_drug_country: Option<String>,
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
struct SenderInformationDoc {
	id: String,
	case_id: String,
	source_sender_presave_id: Option<String>,
	sender_type: Option<String>,
	health_professional_type_kr1: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	person_title: Option<String>,
	person_given_name: Option<String>,
	person_middle_name: Option<String>,
	person_family_name: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
	created_at: String,
	updated_at: String,
	created_by: String,
	updated_by: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SenderInformationForCreateDoc {
	case_id: String,
	source_sender_presave_id: Option<String>,
	sender_type: Option<String>,
	health_professional_type_kr1: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	person_title: Option<String>,
	person_given_name: Option<String>,
	person_middle_name: Option<String>,
	person_family_name: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct SenderInformationForUpdateDoc {
	source_sender_presave_id: Option<String>,
	sender_type: Option<String>,
	health_professional_type_kr1: Option<String>,
	organization_name: Option<String>,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	person_title: Option<String>,
	person_given_name: Option<String>,
	person_middle_name: Option<String>,
	person_family_name: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
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
	fulfil_expedited_criteria: Option<bool>,
	fulfil_expedited_criteria_null_flavor: Option<String>,
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	worldwide_unique_id: Option<String>,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
	other_case_identifiers_exist: Option<bool>,
	other_case_identifiers_exist_null_flavor: Option<String>,
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
	fulfil_expedited_criteria: Option<bool>,
	fulfil_expedited_criteria_null_flavor: Option<String>,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
	other_case_identifiers_exist: Option<bool>,
	other_case_identifiers_exist_null_flavor: Option<String>,
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
	fulfil_expedited_criteria_null_flavor: Option<String>,
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	worldwide_unique_id: Option<String>,
	first_sender_type: Option<String>,
	additional_documents_available: Option<bool>,
	other_case_identifiers_exist: Option<bool>,
	other_case_identifiers_exist_null_flavor: Option<String>,
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
	source_narrative_presave_id: Option<String>,
	case_narrative: String,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
	additional_information: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct NarrativeInformationForUpdateDoc {
	source_narrative_presave_id: Option<String>,
	case_narrative: Option<String>,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
	additional_information: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct NarrativePreviewRequestDoc {
	template: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct NarrativePreviewTokenDoc {
	code: String,
	resolved: bool,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct NarrativePreviewResponseDoc {
	rendered: String,
	tokens: Vec<NarrativePreviewTokenDoc>,
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
	batch_result: String,
	message_result: Option<String>,
	xml_bytes: usize,
	submitted_by: String,
	submitted_by_email: Option<String>,
	submitted_at: String,
	latest_ack_received_at: Option<String>,
	acknowledged_date: Option<String>,
	latest_event_type: Option<String>,
	icsr_count: i32,
	data_file_name: String,
	data_file_download_url: String,
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
	get,
	path = "/api/users/me/profile",
	tag = "users",
	security(
		("auth_token" = [])
	),
	responses(
		(status = 200, description = "Current user profile with routing and derived capabilities", body = CurrentUserProfileResponse),
		(status = 401, description = "Authentication required", body = ErrorResponse)
	)
)]
fn get_current_user_profile() {}

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
	path = "/api/cases/{case_id}/editor/shell",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Case editor shell with case header, workflow, and permissions", body = CaseEditorShellDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_shell() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/CI",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Case identification editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_ci() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/CI",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Case identification page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_ci_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/CI",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated case identification page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_ci_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/RP",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Reporter page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_rp_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/RP",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated reporter page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_rp_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/SD",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Sender page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_sd_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/SD",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated sender page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_sd_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/LR",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Literature references page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_lr_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/LR",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated literature references page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_lr_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/SI",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Study information page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_si_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/SI",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated study information page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_si_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/DM",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Patient demographics page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_dm_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/DM",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated patient demographics page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_dm_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/NR",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Narrative page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_nr_page() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/NR",
	tag = "case-editor",
	security(("auth_token" = [])),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated narrative page projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 400, description = "Invalid patch or authority context", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn patch_editor_nr_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/DH",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Past drug history page row projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_dh_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/AE",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Reaction page row projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_ae_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/LB",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Lab test page row projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_lb_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/DG",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Drug page row projection", body = CaseEditorPageProjectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_dg_page() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/DH/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("row_id" = String, Path, description = "Past drug history row ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Past drug history page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row ID", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn get_editor_dh_page_row() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/AE/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("row_id" = String, Path, description = "Reaction row ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Reaction page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row ID", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn get_editor_ae_page_row() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/LB/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("row_id" = String, Path, description = "Test result row ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Lab test page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row ID", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn get_editor_lb_page_row() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/pages/DG/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("row_id" = String, Path, description = "Drug row ID"),
		("authorities" = Option<String>, Query, description = "Comma-separated validation/render authorities: ich,fda,mfds")
	),
	responses(
		(status = 200, description = "Drug page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row ID", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn get_editor_dg_page_row() {}

#[utoipa::path(
	post,
	path = "/api/cases/{case_id}/editor/pages/{section}/rows",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("section" = String, Path, description = "Repeatable editor section: DH, AE, LB, or DG")
	),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 201, description = "Created repeatable page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row payload", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn create_editor_repeatable_page_row() {}

#[utoipa::path(
	patch,
	path = "/api/cases/{case_id}/editor/pages/{section}/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("section" = String, Path, description = "Repeatable editor section: DH, AE, LB, or DG"),
		("row_id" = String, Path, description = "Repeatable row ID")
	),
	request_body = CaseEditorPagePatchRequestDoc,
	responses(
		(status = 200, description = "Updated repeatable page row detail", body = CaseEditorRowDetailResponseDoc),
		(status = 400, description = "Invalid row ID or payload", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn patch_editor_repeatable_page_row() {}

#[utoipa::path(
	delete,
	path = "/api/cases/{case_id}/editor/pages/{section}/rows/{row_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("section" = String, Path, description = "Repeatable editor section: DH, AE, LB, or DG"),
		("row_id" = String, Path, description = "Repeatable row ID")
	),
	responses(
		(status = 204, description = "Deleted repeatable page row"),
		(status = 400, description = "Invalid row ID", body = ErrorResponse),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or row not found", body = ErrorResponse)
	)
)]
fn delete_editor_repeatable_page_row() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/RP",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Reporter editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_rp() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/SD",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Sender editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_sd() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/LR",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Literature references editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_lr() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/SI",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Study information editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_si() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/DM",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Patient demographics editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_dm() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/NR",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Narrative editor payload", body = CaseEditorDirectSectionResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn get_editor_nr() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/DH/list",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Past drug history editor rows", body = CaseEditorDhListResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn list_editor_dh() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/DH/{past_drug_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("past_drug_id" = String, Path, description = "Past drug history row ID")
	),
	responses(
		(status = 200, description = "Past drug history editor row payload", body = CaseEditorRowDetailResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or past drug history row not found", body = ErrorResponse)
	)
)]
fn get_editor_dh() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/AE/list",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Reaction editor rows", body = CaseEditorAeListResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn list_editor_ae() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/AE/{reaction_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("reaction_id" = String, Path, description = "Reaction row ID")
	),
	responses(
		(status = 200, description = "Reaction editor row payload", body = CaseEditorRowDetailResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or reaction row not found", body = ErrorResponse)
	)
)]
fn get_editor_ae() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/LB/list",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Lab test editor rows", body = CaseEditorLbListResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn list_editor_lb() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/LB/{test_result_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("test_result_id" = String, Path, description = "Test result row ID")
	),
	responses(
		(status = 200, description = "Lab test editor row payload", body = CaseEditorRowDetailResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or test result row not found", body = ErrorResponse)
	)
)]
fn get_editor_lb() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/DG/list",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	responses(
		(status = 200, description = "Drug editor rows", body = CaseEditorDgListResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case not found", body = ErrorResponse)
	)
)]
fn list_editor_dg() {}

#[utoipa::path(
	get,
	path = "/api/cases/{case_id}/editor/DG/{drug_id}",
	tag = "case-editor",
	security(
		("auth_token" = [])
	),
	params(
		("case_id" = String, Path, description = "Case ID"),
		("drug_id" = String, Path, description = "Drug row ID")
	),
	responses(
		(status = 200, description = "Drug editor row payload with nested drug children", body = CaseEditorRowDetailResponseDoc),
		(status = 403, description = "Permission denied", body = ErrorResponse),
		(status = 404, description = "Case or drug row not found", body = ErrorResponse)
	)
)]
fn get_editor_dg() {}

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
	post,
	path = "/api/cases/{case_id}/narrative/preview",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("case_id" = String, Path, description = "Case ID")),
	request_body = NarrativePreviewRequestDoc,
	responses((status = 200, description = "Rendered narrative preview", body = NarrativePreviewResponseDoc))
)]
fn preview_case_narrative() {}

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
		("authority" = Option<String>, Query, description = "Validation authority override: ich, fda, or mfds")
	),
	responses((status = 200, description = "Case validation report", body = GenericDataResponse))
)]
fn validate_case() {}

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
		("authority" = Option<String>, Query, description = "Export authority: ich, fda, or mfds. Must be selected on the case.")
	),
	responses((status = 200, description = "Case XML export"))
)]
fn export_case_xml() {}

#[utoipa::path(
	get,
	path = "/api/cases/{id}/export/cioms.pdf",
	tag = "case-subresources",
	security(
		("auth_token" = [])
	),
	params(("id" = String, Path, description = "Case ID")),
	responses((status = 200, description = "Case CIOMS PDF export"))
)]
fn export_case_cioms_pdf() {}

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

#[cfg(test)]
mod tests {
	use super::ApiDoc;
	use utoipa::OpenApi;

	#[test]
	fn case_editor_page_projection_documents_authorities_contract() {
		let doc = serde_json::to_value(ApiDoc::openapi()).expect("openapi json");
		let schema = &doc["components"]["schemas"]
			["CaseEditorPageProjectionResponseDoc"]["properties"];
		assert!(
			schema.get("authorities").is_some(),
			"page projection response schema must expose authorities: {schema}"
		);

		let ci_get_params = doc["paths"]["/api/cases/{case_id}/editor/pages/CI"]
			["get"]["parameters"]
			.as_array()
			.expect("CI GET params");
		assert!(
			ci_get_params.iter().any(|param| param["name"] == "authorities"),
			"CI page projection GET must document authorities query parameter: {ci_get_params:?}"
		);
		assert!(
			ci_get_params
				.iter()
				.all(|param| param["name"] != ["appen", "dix"].concat()),
			"CI page projection GET must not document legacy single-authority compatibility: {ci_get_params:?}"
		);

		let patch_schema = &doc["components"]["schemas"]
			["CaseEditorPagePatchRequestDoc"]["properties"];
		assert!(
			patch_schema.get("authorities").is_some(),
			"page patch request schema must expose authorities body field: {patch_schema}"
		);
		assert!(
			patch_schema.get(&["appen", "dix"].concat()).is_none(),
			"page patch request schema must not expose legacy single-authority body field: {patch_schema}"
		);

		let row_schema = &doc["components"]["schemas"]
			["CaseEditorRowDetailResponseDoc"]["properties"];
		assert!(
			row_schema.get("authorities").is_some(),
			"page row response schema must expose authorities field: {row_schema}"
		);
		assert!(
			row_schema
				.get(&["focused", "App", "endix"].concat())
				.is_none(),
			"page row response schema must not expose legacy focus field: {row_schema}"
		);

		let shell_schema =
			&doc["components"]["schemas"]["CaseEditorShellDoc"]["properties"];
		assert!(
			shell_schema.get("appendices").is_none(),
			"case editor shell schema must not expose case-level authority metadata: {shell_schema}"
		);
	}

	#[test]
	fn drug_information_documents_dg_kr_product_fields() {
		let doc = serde_json::to_value(ApiDoc::openapi()).expect("openapi json");
		let schemas = &doc["components"]["schemas"];

		for schema_name in [
			"DrugInformationDoc",
			"DrugInformationForCreateDoc",
			"DrugInformationForUpdateDoc",
		] {
			let properties = &schemas[schema_name]["properties"];
			assert!(
				properties.get("mpid").is_some(),
				"{schema_name} must expose base mpid: {properties}"
			);
			assert!(
				properties.get("mpid_version").is_some(),
				"{schema_name} must expose base mpid_version: {properties}"
			);
			assert!(
				properties.get("mfds_mpid_version").is_some(),
				"{schema_name} must expose KR mfds_mpid_version: {properties}"
			);
			assert!(
				properties.get("mfds_mpid").is_some(),
				"{schema_name} must expose KR mfds_mpid: {properties}"
			);
		}
	}
}
