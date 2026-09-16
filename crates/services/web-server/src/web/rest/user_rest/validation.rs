use super::*;

const PASSWORD_MIN_CHARS: usize = 15;
const PASSWORD_MAX_CHARS: usize = 128;

pub(super) fn validate_username(username: &str) -> Result<()> {
	if username.chars().count() > USERNAME_MAX_LEN {
		return Err(Error::BadRequest {
			message: "username must be 128 characters or fewer".to_string(),
		});
	}
	Ok(())
}

pub(super) fn validate_email(email: &str) -> Result<()> {
	if email.chars().count() > EMAIL_MAX_LEN {
		return Err(Error::BadRequest {
			message: "email must be 255 characters or fewer".to_string(),
		});
	}
	Ok(())
}

pub(super) fn normalize_email_input(email: String) -> Result<String> {
	let email = email.trim().to_string();
	validate_email(&email)?;
	Ok(email)
}

pub(super) fn normalize_optional_email_input(
	email: Option<String>,
) -> Result<Option<String>> {
	email.map(normalize_email_input).transpose()
}

pub(super) fn normalize_optional_username_input(
	username: Option<String>,
) -> Result<Option<String>> {
	username
		.map(|value| {
			let username = value.trim().to_string();
			validate_username(&username)?;
			Ok(username)
		})
		.transpose()
}

pub(super) fn parse_scope_input(
	value: Option<ScopeListInput>,
) -> Option<Vec<String>> {
	match value {
		None => None,
		Some(ScopeListInput::List(values)) => Some(values),
		Some(ScopeListInput::Encoded(raw)) => {
			serde_json::from_str::<Vec<String>>(&raw).ok().or_else(|| {
				Some(
					raw.split(',')
						.map(|value| value.trim().to_string())
						.filter(|value| !value.is_empty())
						.collect::<Vec<_>>(),
				)
			})
		}
	}
}

pub(super) fn serialize_scope_input(
	value: Option<ScopeListInput>,
) -> Option<String> {
	parse_scope_input(value).map(|values| {
		let values = values
			.into_iter()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.collect::<Vec<_>>();
		serde_json::json!(values).to_string()
	})
}

pub(super) fn validate_uuid_scope(
	field: &str,
	input: &Option<ScopeListInput>,
) -> Result<()> {
	for value in parse_scope_input(input.clone()).unwrap_or_default() {
		Uuid::parse_str(value.trim()).map_err(|_| Error::BadRequest {
			message: format!("{field} accepts UUID values only"),
		})?;
	}
	Ok(())
}

pub(super) async fn validate_scope_hierarchy(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
	sender_ids: &[String],
	product_ids: &[String],
	study_ids: &[String],
) -> Result<()> {
	let sender_ids = sender_ids
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>();
	let product_ids = product_ids
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>();
	let study_ids = study_ids
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>();

	let violation = lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_one(
				sqlx::query_as::<_, (Option<String>,)>(
					"SELECT validate_scope_assignment($1, $2, $3, $4)",
				)
				.bind(organization_id)
				.bind(&sender_ids)
				.bind(&product_ids)
				.bind(&study_ids),
			)
			.await
			.map(|row| row.0)
			.map_err(|error| Error::Model(error.into()))
		})
	})
	.await?;

	match violation.as_deref() {
		None => Ok(()),
		Some("sender_not_found") => Err(Error::BadRequest {
			message: "access_sender_ids must reference active Senders in the organization"
				.to_string(),
		}),
		Some("product_sender_mismatch") => Err(Error::BadRequest {
			message: "access_product_ids must reference Products belonging to access_sender_ids"
				.to_string(),
		}),
		Some("study_product_mismatch") => Err(Error::BadRequest {
			message: "access_study_ids must reference Studies belonging to access_product_ids"
				.to_string(),
		}),
		Some("study_sender_mismatch") => Err(Error::BadRequest {
			message: "access_study_ids must reference Studies belonging to access_sender_ids"
				.to_string(),
		}),
		Some(violation) => Err(Error::BadRequest {
			message: format!("invalid scope assignment: {violation}"),
		}),
	}
}

pub(super) fn scope_values_for_update(
	value: Option<&ScopeListInput>,
	current: Option<&str>,
) -> Vec<String> {
	value
		.and_then(|value| parse_scope_input(Some(value.clone())))
		.or_else(|| {
			parse_scope_input(
				current.map(|value| ScopeListInput::Encoded(value.to_string())),
			)
		})
		.unwrap_or_default()
}

pub(super) fn validate_optional_uuid_identifier(
	field: &str,
	value: Option<&str>,
) -> Result<()> {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(());
	};
	Uuid::parse_str(value).map_err(|_| Error::BadRequest {
		message: format!("{field} accepts a UUID value only"),
	})?;
	Ok(())
}

pub(super) fn role_display_name(role: &str) -> String {
	let canonical = canonical_role(role);
	built_in_role_metadata(&canonical)
		.map(|metadata| metadata.display_name.to_string())
		.unwrap_or_else(|| canonical.replace('_', " "))
}

pub(super) fn role_metadata(
	role: &str,
	custom_name: Option<&str>,
) -> UserRoleMetadata {
	let canonical_role_id = canonical_role(role);
	let built_in = built_in_role_metadata(&canonical_role_id);
	UserRoleMetadata {
		display_name: built_in
			.map(|metadata| metadata.display_name.to_string())
			.or_else(|| {
				custom_name
					.map(str::trim)
					.filter(|name| !name.is_empty())
					.map(str::to_string)
			})
			.unwrap_or_else(|| role_display_name(&canonical_role_id)),
		canonical_role_id: canonical_role_id.clone(),
		is_builtin: built_in.is_some(),
		is_editable: built_in.is_none(),
		is_sponsor_admin: built_in.is_some_and(|metadata| metadata.sponsor_admin),
		is_operational: built_in.is_none_or(|metadata| metadata.operational),
	}
}

pub(super) fn normalize_user_role(role: Option<String>) -> Option<String> {
	role.map(|role| canonical_role(&role))
		.filter(|role| !role.trim().is_empty())
}

pub(super) fn sponsor_admin_role_error() -> Error {
	Error::BadRequest {
		message: "sponsor_admin_cro can only be assigned in CRO organizations; sponsor_admin_company can only be assigned in Pharmaceutical company organizations".to_string(),
	}
}

pub(super) fn is_sponsor_admin_role(role: &str) -> bool {
	matches!(role, ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY)
}

pub(super) fn is_built_in_admin_role(role: &str) -> bool {
	matches!(
		role,
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	)
}

pub(super) fn sponsor_admin_singleton_error() -> Error {
	Error::BadRequest {
		message:
			"Only one Sponsor Administrator can be assigned for an organization"
				.to_string(),
	}
}

pub(super) async fn validate_permission_profile_role_for_org(
	ctx: &Ctx,
	mm: &ModelManager,
	role: Option<&str>,
) -> Result<()> {
	let Some(role) = role else {
		return Ok(());
	};
	if matches!(
		role,
		ROLE_SYSTEM_ADMIN
			| ROLE_SPONSOR_ADMIN_CRO
			| ROLE_SPONSOR_ADMIN_COMPANY
			| ROLE_USER
	) {
		return Ok(());
	}
	let profile_id = Uuid::parse_str(role).map_err(|_| Error::BadRequest {
		message: "permission profile must be registered before creating users"
			.to_string(),
	})?;
	let exists =
		PermissionProfileBmc::active_custom_exists_in_org(ctx, mm, profile_id)
			.await
			.map_err(Error::Model)?;
	if !exists {
		return Err(Error::BadRequest {
			message: "permission profile must be registered before creating users"
				.to_string(),
		});
	}
	Ok(())
}

pub(super) async fn validate_sponsor_admin_role_for_org(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
	role: Option<&str>,
) -> Result<()> {
	let Some(role) = role else {
		return Ok(());
	};
	if !is_sponsor_admin_role(role) {
		return Ok(());
	}
	let organization: Organization =
		OrganizationBmc::get(ctx, mm, organization_id).await?;
	match (role, organization.org_type.as_deref()) {
		(ROLE_SPONSOR_ADMIN_CRO, Some(ORG_TYPE_CRO))
		| (ROLE_SPONSOR_ADMIN_COMPANY, Some(ORG_TYPE_PHARMACEUTICAL_COMPANY)) => Ok(()),
		_ => Err(sponsor_admin_role_error()),
	}
}

pub(super) async fn validate_single_sponsor_admin_for_org(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
	role: Option<&str>,
	exclude_user_id: Option<Uuid>,
) -> Result<()> {
	let Some(role) = role else {
		return Ok(());
	};
	if !is_sponsor_admin_role(role) {
		return Ok(());
	}

	let count = lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_one(
				sqlx::query_as::<_, (i64,)>(
					r#"
				SELECT COUNT(*)
				FROM users
				WHERE organization_id = $1
				  AND active = true
				  AND role IN ($2, $3)
				  AND ($4::uuid IS NULL OR id <> $4)
				"#,
				)
				.bind(organization_id)
				.bind(ROLE_SPONSOR_ADMIN_CRO)
				.bind(ROLE_SPONSOR_ADMIN_COMPANY)
				.bind(exclude_user_id),
			)
			.await
			.map_err(|err| Error::Model(err.into()))
		})
	})
	.await?
	.0;

	if count > 0 {
		return Err(sponsor_admin_singleton_error());
	}
	Ok(())
}

pub(super) fn validate_new_password(password: &str) -> Result<()> {
	let length = password.chars().count();
	if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&length) {
		return Err(Error::BadRequest {
			message: format!(
				"password must be between {PASSWORD_MIN_CHARS} and {PASSWORD_MAX_CHARS} characters"
			),
		});
	}
	if password.chars().any(char::is_control) {
		return Err(Error::BadRequest {
			message: "password must not contain control characters".to_string(),
		});
	}
	Ok(())
}

pub(super) fn initial_password(pwd_clear: Option<String>) -> Result<String> {
	let password = pwd_clear
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| "welcome".to_string());
	if password == "welcome" {
		return Ok(password);
	}
	validate_new_password(&password)?;
	Ok(password)
}

pub(super) fn deserialize_access_datetime_option<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<OffsetDateTime>, D::Error>
where
	D: Deserializer<'de>,
{
	let value = Option::<String>::deserialize(deserializer)?;
	value
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(parse_access_datetime)
		.transpose()
		.map_err(de::Error::custom)
}

pub(super) fn deserialize_patch_access_datetime_option<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<Option<OffsetDateTime>>, D::Error>
where
	D: Deserializer<'de>,
{
	deserialize_access_datetime_option(deserializer).map(Some)
}

pub(super) fn parse_access_datetime(
	value: &str,
) -> std::result::Result<OffsetDateTime, String> {
	if let Ok(datetime) =
		OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
	{
		return Ok(datetime);
	}

	for format in [
		"[year]-[month]-[day]T[hour]:[minute]",
		"[year]-[month]-[day]T[hour]:[minute]:[second]",
	] {
		let description = format_description::parse(format)
			.map_err(|err| format!("invalid datetime parser format: {err}"))?;
		if let Ok(datetime) = PrimitiveDateTime::parse(value, &description) {
			return Ok(datetime.assume_utc());
		}
	}

	Err("expected RFC3339 or datetime-local format".to_string())
}

pub(super) fn user_is_effectively_active(user: &User) -> bool {
	if !user.active {
		return false;
	}
	let now = OffsetDateTime::now_utc();
	if user.access_start_at.is_some_and(|start_at| start_at > now) {
		return false;
	}
	if user.access_end_at.is_some_and(|end_at| end_at < now) {
		return false;
	}
	true
}

pub(super) fn has_sender_scope_assignment(
	active_sender_identifier: &Option<String>,
	access_sender_ids: &Option<ScopeListInput>,
) -> bool {
	active_sender_identifier
		.as_deref()
		.is_some_and(|value| !value.trim().is_empty())
		|| parse_scope_input(access_sender_ids.clone()).is_some_and(|values| {
			values.iter().any(|value| !value.trim().is_empty())
		})
}

pub(super) fn sender_scope_assignment_forbidden() -> Error {
	Error::AccessDenied {
		required_role: "sender_scope_assignment_sponsor_admin".to_string(),
	}
}

#[cfg(test)]
mod uuid_scope_tests {
	use super::*;

	#[test]
	fn presave_scope_values_must_be_uuids() {
		let valid_id = Uuid::new_v4().to_string();
		assert!(validate_uuid_scope(
			"access_sender_ids",
			&Some(ScopeListInput::List(vec![valid_id])),
		)
		.is_ok());
		assert!(validate_uuid_scope(
			"access_sender_ids",
			&Some(ScopeListInput::List(vec!["Sender Org A".to_string()])),
		)
		.is_err());
		assert!(validate_optional_uuid_identifier(
			"active_sender_identifier",
			Some("Sender Org A"),
		)
		.is_err());
	}

	#[test]
	fn empty_sender_scope_is_not_an_assignment() {
		assert!(!has_sender_scope_assignment(
			&None,
			&Some(ScopeListInput::List(Vec::new())),
		));
		assert!(!has_sender_scope_assignment(
			&Some("  ".to_string()),
			&Some(ScopeListInput::Encoded(String::new())),
		));
	}

	#[test]
	fn populated_sender_scope_is_an_assignment() {
		assert!(has_sender_scope_assignment(
			&Some(Uuid::new_v4().to_string()),
			&None,
		));
		assert!(has_sender_scope_assignment(
			&None,
			&Some(ScopeListInput::List(vec![Uuid::new_v4().to_string()])),
		));
	}

	#[test]
	fn empty_scope_serializes_as_explicit_clear() {
		assert_eq!(
			serialize_scope_input(Some(ScopeListInput::List(Vec::new()))),
			Some("[]".to_string())
		);
		assert_eq!(serialize_scope_input(None), None);
	}

	#[test]
	fn initial_password_defaults_to_welcome_or_meets_the_shared_policy() {
		assert_eq!(initial_password(None).unwrap(), "welcome");
		assert_eq!(initial_password(Some("  ".to_string())).unwrap(), "welcome");
		assert!(initial_password(Some("too-short".to_string())).is_err());
		assert_eq!(
			initial_password(Some("long-enough-password".to_string())).unwrap(),
			"long-enough-password"
		);
	}
}
