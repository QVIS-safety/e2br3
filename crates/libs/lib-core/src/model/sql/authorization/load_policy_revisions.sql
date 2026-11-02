SELECT o.revision, p.revision
FROM organization_policy_state o
JOIN principal_authorization_state p ON p.organization_id = o.organization_id
WHERE o.organization_id = $1 AND p.user_id = $2
