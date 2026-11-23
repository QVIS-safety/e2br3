use lib_core::authorization::{
	grant_ids_for_menu_privileges, policy_registry, AdminMenuPrivilege,
	Availability, GrantUiField, PrivilegeAdapterError,
};

fn privilege(menu_key: &str, field: GrantUiField) -> AdminMenuPrivilege {
	AdminMenuPrivilege {
		menu_key: menu_key.to_string(),
		can_read: field == GrantUiField::CanRead,
		can_edit: field == GrantUiField::CanEdit,
		can_review: field == GrantUiField::CanReview,
		can_lock: field == GrantUiField::CanLock,
	}
}

fn legacy_field(value: &str) -> GrantUiField {
	match value {
		"read" => GrantUiField::CanRead,
		"edit" => GrantUiField::CanEdit,
		"review" => GrantUiField::CanReview,
		"lock" => GrantUiField::CanLock,
		other => panic!("unsupported legacy field {other:?}"),
	}
}

#[test]
fn every_pdf_row_maps_to_exactly_its_canonical_grant_or_is_reserved(
) -> Result<(), Box<dyn std::error::Error>> {
	let registry = policy_registry()?;
	let grants = registry.grants().collect::<Vec<_>>();
	assert_eq!(grants.len(), 18, "the UI specification defines 18 rows");

	for grant in grants {
		let row =
			privilege(grant.ui_binding.menu_key.as_str(), grant.ui_binding.field);
		let actual = grant_ids_for_menu_privileges(&[row], false).unwrap();
		match grant.availability {
			Availability::Implemented => {
				assert_eq!(
					actual.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
					vec![grant.id.as_str()],
					"PDF row {} must have one canonical grant",
					grant.id
				);
			}
			Availability::Reserved => assert!(
				actual.is_empty(),
				"reserved PDF row {} must never become assignable",
				grant.id
			),
		}
	}
	Ok(())
}

#[test]
fn migration_aliases_translate_to_their_canonical_grants(
) -> Result<(), Box<dyn std::error::Error>> {
	let registry = policy_registry()?;
	for alias in registry.legacy_aliases() {
		let (menu_key, field) = alias
			.legacy_id
			.as_str()
			.rsplit_once('.')
			.expect("legacy aliases use menu.field form");
		let actual = grant_ids_for_menu_privileges(
			&[privilege(menu_key, legacy_field(field))],
			true,
		)
		.unwrap();
		assert_eq!(
			actual.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
			vec![alias.grant_id.as_str()],
			"legacy alias {} must translate only during migration",
			alias.legacy_id
		);
	}
	Ok(())
}

#[test]
fn current_role_writes_reject_migration_only_menu_aliases() {
	for menu_key in ["case.qc", "case.lock", "export", "submission"] {
		let error = grant_ids_for_menu_privileges(
			&[privilege(menu_key, GrantUiField::CanEdit)],
			false,
		)
		.unwrap_err();
		assert_eq!(
			error,
			PrivilegeAdapterError::UnknownMenu {
				menu_key: menu_key.to_string()
			}
		);
	}
}

#[test]
fn canonical_implications_expand_only_in_the_registry(
) -> Result<(), Box<dyn std::error::Error>> {
	let direct = grant_ids_for_menu_privileges(
		&[privilege("admin", GrantUiField::CanEdit)],
		false,
	)
	.unwrap();
	assert_eq!(
		direct.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
		vec!["admin.edit"]
	);

	let effective = policy_registry()?
		.effective_grants(direct.iter().map(|id| id.as_str()))
		.unwrap();
	assert!(effective.iter().any(|id| id.as_str() == "admin.edit"));
	assert!(effective.iter().any(|id| id.as_str() == "admin.read"));
	Ok(())
}

#[test]
fn unknown_rows_are_rejected_even_when_all_flags_are_disabled() {
	let error = grant_ids_for_menu_privileges(
		&[AdminMenuPrivilege {
			menu_key: "unknown_menu".to_string(),
			can_read: false,
			can_edit: false,
			can_review: false,
			can_lock: false,
		}],
		true,
	)
	.unwrap_err();
	assert_eq!(
		error,
		PrivilegeAdapterError::UnknownMenu {
			menu_key: "unknown_menu".to_string()
		}
	);
}
