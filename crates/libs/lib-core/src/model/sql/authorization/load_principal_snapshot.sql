SELECT
	a.role_id,
	r.organization_id AS role_organization_id,
	r.active AS role_active,
	r.deleted_at AS role_deleted_at,
	u.active AS user_active,
	m.active AS membership_active,
	a.active AS assignment_active,
	o.active AS organization_active,
	lower(o.org_type) AS organization_type,
	u.access_start_at,
	u.access_end_at,
	u.access_sender_ids,
	u.access_product_ids,
	u.access_study_ids,
	u.access_blind_allowed,
	u.active_sender_identifier,
	os.revision AS organization_revision,
	ps.revision AS principal_revision,
	cs.catalog_hash AS stored_catalog_hash
FROM users u
JOIN user_organization_memberships m
  ON m.user_id = u.id AND m.organization_id = $2
JOIN organizations o ON o.id = m.organization_id
JOIN user_role_assignments a
  ON a.user_id = u.id AND a.organization_id = m.organization_id
JOIN authorization_roles r ON r.id = a.role_id
LEFT JOIN organization_policy_state os
  ON os.organization_id = m.organization_id
LEFT JOIN principal_authorization_state ps
  ON ps.user_id = u.id AND ps.organization_id = m.organization_id
LEFT JOIN authorization_catalog_state cs ON cs.singleton
WHERE u.id = $1
