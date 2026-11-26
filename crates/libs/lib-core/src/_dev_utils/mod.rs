pub fn fx_org_id() -> uuid::Uuid {
	// NOTE: This org_id is created via db/admin/00-recreate-db.sql
	uuid::uuid!("00000000-0000-0000-0000-000000000001")
}
