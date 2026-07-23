use super::*;

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
	parse_scope_input(value).and_then(|values| {
		let values = values
			.into_iter()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.collect::<Vec<_>>();
		if values.is_empty() {
			None
		} else {
			Some(serde_json::json!(values).to_string())
		}
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
	match canonical_role(role).as_str() {
		ROLE_SYSTEM_ADMIN => "System Administrator".to_string(),
		ROLE_SPONSOR_ADMIN_CRO => "Sponsor Administrator (CRO)".to_string(),
		ROLE_SPONSOR_ADMIN_COMPANY => {
			"Sponsor Administrator (Pharmaceutical Company)".to_string()
		}
		other => other.replace('_', " "),
	}
}

pub(super) fn role_metadata(role: &str, _unused: Option<&str>) -> UserRoleMetadata {
	let canonical_role_id = canonical_role(role);
	let is_builtin = matches!(
		canonical_role_id.as_str(),
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	);
	let is_sponsor_admin = matches!(
		canonical_role_id.as_str(),
		ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	);
	UserRoleMetadata {
		display_name: role_display_name(&canonical_role_id),
		canonical_role_id: canonical_role_id.clone(),
		is_builtin,
		is_editable: !is_builtin,
		is_sponsor_admin,
		is_operational: canonical_role_id != ROLE_SYSTEM_ADMIN,
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

pub(super) fn sponsor_admin_mutation_error() -> Error {
	Error::BadRequest {
		message:
			"Sponsor Administrator users can only be changed by a System Administrator"
				.to_string(),
	}
}

pub(super) fn sponsor_admin_singleton_error() -> Error {
	Error::BadRequest {
		message:
			"Only one Sponsor Administrator can be assigned for an organization"
				.to_string(),
	}
}

pub(super) fn validate_sponsor_admin_assignment_authority(
	ctx: &Ctx,
	role: Option<&str>,
) -> Result<()> {
	if role.is_some_and(is_sponsor_admin_role) && !ctx.is_system_admin() {
		return Err(Error::BadRequest {
			message: "Sponsor Administrator roles can only be assigned by a System Administrator".to_string(),
		});
	}
	Ok(())
}

pub(super) fn validate_create_role_selection(role: Option<&str>) -> Result<()> {
	match role {
		Some(ROLE_USER) | None => Err(Error::BadRequest {
			message: "permission profile selection is required".to_string(),
		}),
		_ => Ok(()),
	}
}

pub(super) fn validate_update_role_selection(role: Option<&str>) -> Result<()> {
	match role {
		Some(ROLE_USER) => Err(Error::BadRequest {
			message: "permission profile selection is required".to_string(),
		}),
		_ => Ok(()),
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
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
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

pub(super) fn validate_existing_sponsor_admin_mutation(
	ctx: &Ctx,
	user: &User,
) -> Result<()> {
	if is_sponsor_admin_role(&user.role) && !ctx.is_system_admin() {
		return Err(sponsor_admin_mutation_error());
	}
	Ok(())
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

pub(super) fn initial_password(pwd_clear: Option<String>) -> String {
	pwd_clear
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| "welcome".to_string())
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
	active_sender_identifier.is_some() || access_sender_ids.is_some()
}

pub(super) fn sender_scope_assignment_forbidden_for_ctx(ctx: &Ctx) -> bool {
	!ctx.is_cro_sponsor_admin()
}

pub(super) fn sender_scope_assignment_forbidden() -> Error {
	Error::AccessDenied {
		required_role: "sender_scope_assignment_cro_admin".to_string(),
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
}
