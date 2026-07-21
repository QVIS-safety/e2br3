use crate::authorization::{InvalidationDomain, PolicyRegistry};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizationRevisions {
	pub organization: i64,
	pub principal: i64,
}

pub struct RevisionRepository;

impl RevisionRepository {
	pub async fn load(
		pool: &Pool<Postgres>,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<AuthorizationRevisions, sqlx::Error> {
		let (organization, principal) = sqlx::query_as::<_, (i64, i64)>(
			"SELECT o.revision, p.revision FROM organization_policy_state o JOIN principal_authorization_state p ON p.organization_id = o.organization_id WHERE o.organization_id = $1 AND p.user_id = $2",
		)
		.bind(organization_id)
		.bind(user_id)
		.fetch_one(pool)
		.await?;
		Ok(AuthorizationRevisions {
			organization,
			principal,
		})
	}

	pub async fn lock(
		transaction: &mut Transaction<'_, Postgres>,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<AuthorizationRevisions, sqlx::Error> {
		let (organization, principal) = sqlx::query_as::<_, (i64, i64)>(
			"SELECT o.revision, p.revision FROM organization_policy_state o JOIN principal_authorization_state p ON p.organization_id = o.organization_id WHERE o.organization_id = $1 AND p.user_id = $2 FOR UPDATE OF o, p",
		)
		.bind(organization_id)
		.bind(user_id)
		.fetch_one(&mut **transaction)
		.await?;
		Ok(AuthorizationRevisions {
			organization,
			principal,
		})
	}

	pub async fn verify_fact_triggers(
		pool: &Pool<Postgres>,
		registry: &PolicyRegistry,
	) -> Result<(), sqlx::Error> {
		for fact in registry.facts() {
			let domain = match fact.invalidation_domain {
				InvalidationDomain::Organization => "organization",
				InvalidationDomain::Principal => "principal",
			};
			let trigger_name = format!("authz_revision_{domain}_{}", fact.table);
			let (definition, columns, trigger_type, enabled, function_name, security_definer) =
				sqlx::query_as::<_, (String, Vec<String>, i16, String, String, bool)>(
					"SELECT pg_get_triggerdef(t.oid), ARRAY(SELECT a.attname FROM unnest(t.tgattr::smallint[]) WITH ORDINALITY AS trigger_column(attnum, ordinal) JOIN pg_attribute a ON a.attrelid = t.tgrelid AND a.attnum = trigger_column.attnum ORDER BY trigger_column.ordinal), t.tgtype, t.tgenabled::text, p.proname, p.prosecdef FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace JOIN pg_proc p ON p.oid = t.tgfoid WHERE n.nspname = current_schema() AND c.relname = $1 AND t.tgname = $2 AND NOT t.tgisinternal",
			)
			.bind(&fact.table)
			.bind(&trigger_name)
			.fetch_optional(pool)
			.await?
			.ok_or_else(|| {
				sqlx::Error::Protocol(format!(
					"missing authorization revision trigger {trigger_name} for fact {}",
					fact.id
				))
			})?;
			let expected_function = match fact.table.as_str() {
				"sender_presaves" | "product_presaves" | "study_presaves" => {
					"authz_revision_organization_scope_definition"
				}
				_ => trigger_name.as_str(),
			};
			let expected_events: i16 = match fact.table.as_str() {
				"organizations" | "users" | "user_organization_memberships" => 16,
				_ => 4 | 8 | 16,
			};
			if enabled != "O"
				|| !security_definer
				|| function_name != expected_function
				|| trigger_type & (4 | 8 | 16 | 32) != expected_events
			{
				return Err(sqlx::Error::Protocol(format!(
					"authorization revision trigger {trigger_name} has an unsafe execution contract"
				)));
			}
			for column in &fact.columns {
				if !columns.contains(column) {
					return Err(sqlx::Error::Protocol(format!(
						"authorization revision trigger {trigger_name} does not cover {}.{} for fact {}",
						fact.table, column, fact.id
					)));
				}
			}
			if !definition.contains("FOR EACH ROW") {
				return Err(sqlx::Error::Protocol(format!(
					"authorization revision trigger {trigger_name} is not row-scoped"
				)));
			}
		}
		for (table, trigger_name) in [
			("organizations", "authz_initialize_organization_revision"),
			(
				"user_organization_memberships",
				"authz_initialize_membership_revision",
			),
		] {
			let valid = sqlx::query_scalar::<_, bool>(
				"SELECT EXISTS (SELECT 1 FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace JOIN pg_proc p ON p.oid = t.tgfoid WHERE n.nspname = current_schema() AND c.relname = $1 AND t.tgname = $2 AND NOT t.tgisinternal AND t.tgenabled = 'O' AND p.proname = $2 AND p.prosecdef AND (t.tgtype & 1) = 1 AND (t.tgtype & 4) = 4)",
			)
			.bind(table)
			.bind(trigger_name)
			.fetch_one(pool)
			.await?;
			if !valid {
				return Err(sqlx::Error::Protocol(format!(
					"missing safe authorization revision initializer {trigger_name}"
				)));
			}
		}
		Ok(())
	}
}
