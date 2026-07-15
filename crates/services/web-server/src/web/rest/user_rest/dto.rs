use super::*;

pub(super) const USERNAME_MAX_LEN: usize = 128;
pub(super) const EMAIL_MAX_LEN: usize = 255;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ScopeListInput {
	List(Vec<String>),
	Encoded(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRoleMetadata {
	pub canonical_role_id: String,
	pub display_name: String,
	pub is_builtin: bool,
	pub is_editable: bool,
	pub is_sponsor_admin: bool,
	pub is_operational: bool,
	pub can_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserScopeView {
	pub assigned_sender_ids: Vec<String>,
	pub assigned_product_ids: Vec<String>,
	pub assigned_study_ids: Vec<String>,
	pub access_blind_allowed: bool,
	pub active_sender_identifier: Option<String>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub access_end_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: String,
	pub role_meta: UserRoleMetadata,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub scope: UserScopeView,
	pub active: bool,
	pub must_change_password: bool,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub last_login_at: Option<OffsetDateTime>,
	#[serde(with = "time::serde::rfc3339")]
	pub created_at: OffsetDateTime,
	#[serde(with = "time::serde::rfc3339")]
	pub updated_at: OffsetDateTime,
	pub created_by: Option<Uuid>,
	pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowUserOptionView {
	pub id: Uuid,
	pub email: String,
	pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationOptionView {
	pub id: Uuid,
	pub name: String,
	#[serde(rename = "type")]
	pub org_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserProfileView {
	pub user: UserView,
	pub active_organization: OrganizationOptionView,
	pub available_organizations: Vec<OrganizationOptionView>,
	pub routing: lib_rest_core::RoutingProfile,
	pub permissions: Vec<String>,
	pub policy_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserOrganizationSelectionView {
	pub active_organization: OrganizationOptionView,
	pub available_organizations: Vec<OrganizationOptionView>,
}

#[derive(Debug, Deserialize)]
pub struct UserForCreateAdminPayload {
	#[serde(default)]
	pub organization_id: Option<Uuid>,
	pub email: String,
	pub username: Option<String>,
	pub pwd_clear: Option<String>,
	pub role: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_end_at: Option<OffsetDateTime>,
	pub active_sender_identifier: Option<String>,
	pub access_sender_ids: Option<ScopeListInput>,
	pub access_product_ids: Option<ScopeListInput>,
	pub access_study_ids: Option<ScopeListInput>,
	pub access_blind_allowed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserForUpdateAdminPayload {
	pub email: Option<String>,
	pub username: Option<String>,
	pub role: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_end_at: Option<OffsetDateTime>,
	pub active_sender_identifier: Option<String>,
	pub access_sender_ids: Option<ScopeListInput>,
	pub access_product_ids: Option<ScopeListInput>,
	pub access_study_ids: Option<ScopeListInput>,
	pub access_blind_allowed: Option<bool>,
	pub active: Option<bool>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub last_login_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingSelectionBody {
	#[serde(default, alias = "sender_id")]
	pub active_sender_identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OrganizationSelectionBody {
	#[serde(alias = "organizationId")]
	pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SetMyPasswordBody {
	pub new_password: String,
}
