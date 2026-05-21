use crate::ctx::{canonical_role, Ctx};
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::store::dbx::Dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{
	FilterNodes, ListOptions, OpValsString, OpValsValue, OrderBy, OrderBys,
};
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Case {
	pub id: Uuid,
	pub organization_id: Uuid,

	// E2B fields
	pub safety_report_id: String,
	pub version: i32,
	pub dg_prd_key: Option<String>,
	pub status: String,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub workflow_status: String,
	pub workflow_assigned_role: Option<String>,
	pub workflow_assigned_user_id: Option<Uuid>,
	pub workflow_due_at: Option<OffsetDateTime>,
	pub workflow_description: Option<String>,
	pub workflow_updated_at: OffsetDateTime,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,

	// Workflow
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
	pub submitted_by: Option<Uuid>,
	pub submitted_at: Option<OffsetDateTime>,

	// Raw imported XML (optional)
	pub raw_xml: Option<Vec<u8>>,
	pub dirty_c: bool,
	pub dirty_d: bool,
	pub dirty_e: bool,
	pub dirty_f: bool,
	pub dirty_g: bool,
	pub dirty_h: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Fields, Deserialize)]
pub struct CaseForCreate {
	pub organization_id: Uuid,
	pub safety_report_id: String,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub version: Option<i32>,
}

#[derive(Fields, Deserialize, Default)]
pub struct CaseForUpdate {
	pub safety_report_id: Option<String>,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub submitted_by: Option<Uuid>,
	pub submitted_at: Option<OffsetDateTime>,
	pub raw_xml: Option<Vec<u8>>,
	pub dirty_c: Option<bool>,
	pub dirty_d: Option<bool>,
	pub dirty_e: Option<bool>,
	pub dirty_f: Option<bool>,
	pub dirty_g: Option<bool>,
	pub dirty_h: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct CaseFilter {
	pub organization_id: Option<OpValsValue>,
	pub safety_report_id: Option<OpValsString>,
	pub status: Option<OpValsString>,
}

fn list_view_order_clause(order_bys: Option<&OrderBys>) -> &'static str {
	let Some(order_by) = order_bys.and_then(|values| values.into_iter().next())
	else {
		return "c.created_at DESC, c.id DESC";
	};

	match order_by {
		OrderBy::Asc(field) => match field.as_str() {
			"created_at" => "c.created_at ASC, c.id ASC",
			"case_no" | "caseNo" | "safety_report_id" => {
				"c.safety_report_id ASC, c.id ASC"
			}
			"date_of_creation" | "dateOfCreation" => {
				"COALESCE(s.transmission_date, c.created_at::date) ASC, c.id ASC"
			}
			"dg_prd_key" | "dgPrdKey" => "c.dg_prd_key ASC NULLS LAST, c.id ASC",
			_ => "c.created_at DESC, c.id DESC",
		},
		OrderBy::Desc(field) => match field.as_str() {
			"created_at" => "c.created_at DESC, c.id DESC",
			"case_no" | "caseNo" | "safety_report_id" => {
				"c.safety_report_id DESC, c.id DESC"
			}
			"date_of_creation" | "dateOfCreation" => {
				"COALESCE(s.transmission_date, c.created_at::date) DESC, c.id DESC"
			}
			"dg_prd_key" | "dgPrdKey" => "c.dg_prd_key DESC NULLS LAST, c.id DESC",
			_ => "c.created_at DESC, c.id DESC",
		},
	}
}

// -- Case domain helpers

/// Returns true when `status` is a recognized case lifecycle value.
pub fn is_valid_case_status(status: &str) -> bool {
	matches!(
		status.trim().to_ascii_lowercase().as_str(),
		"draft"
			| "reviewed"
			| "validated"
			| "locked"
			| "submitted"
			| "deleted"
			| "archived"
			| "nullified"
	)
}

/// Returns true when transitioning `from` → `to` is a permitted lifecycle move.
pub fn is_allowed_case_status_transition(from: &str, to: &str) -> bool {
	let from = from.trim().to_ascii_lowercase();
	let to = to.trim().to_ascii_lowercase();
	if from == to {
		return true;
	}
	match from.as_str() {
		"" | "draft" => matches!(
			to.as_str(),
			"reviewed"
				| "validated"
				| "locked" | "submitted"
				| "deleted" | "archived"
				| "nullified"
		),
		"reviewed" => matches!(
			to.as_str(),
			"draft"
				| "validated"
				| "locked" | "submitted"
				| "deleted" | "archived"
				| "nullified"
		),
		"validated" => matches!(
			to.as_str(),
			"draft"
				| "reviewed" | "locked"
				| "submitted"
				| "deleted" | "archived"
				| "nullified"
		),
		"locked" => matches!(
			to.as_str(),
			"validated" | "submitted" | "deleted" | "archived" | "nullified"
		),
		"submitted" => matches!(to.as_str(), "deleted" | "archived" | "nullified"),
		"deleted" | "archived" => false,
		"nullified" => to == "archived",
		_ => false,
	}
}

/// Returns true when an update touches fields beyond just `status`.
/// Used to block edits on non-editable workflow states.
pub fn update_touches_non_status_fields(case_u: &CaseForUpdate) -> bool {
	case_u.safety_report_id.is_some()
		|| case_u.dg_prd_key.is_some()
		|| case_u.review_receivers_json.is_some()
		|| case_u.workflow_routes_json.is_some()
		|| case_u.mfds_report_type.is_some()
		|| case_u.report_year.is_some()
		|| case_u.source_document_name.is_some()
		|| case_u.source_document_base64.is_some()
		|| case_u.source_document_media_type.is_some()
		|| case_u.submitted_by.is_some()
		|| case_u.submitted_at.is_some()
		|| case_u.raw_xml.is_some()
		|| case_u.dirty_c.is_some()
		|| case_u.dirty_d.is_some()
		|| case_u.dirty_e.is_some()
		|| case_u.dirty_f.is_some()
		|| case_u.dirty_g.is_some()
		|| case_u.dirty_h.is_some()
}

// -- CaseLinkOption (read projection for case-link dropdowns)

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CaseLinkOption {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
	pub transmission_date: Option<Date>,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CaseListViewRow {
	pub no: i64,
	pub case_id: Uuid,
	pub case_no: String,
	pub fu: i32,
	pub date_of_creation: String,
	pub dg_prd_key: String,
	pub warn: String,
	pub wf_status: String,
	pub submission: String,
	pub sae: String,
	pub meddra: String,
	pub ae_term: String,
	pub study_no: String,
	pub subject: String,
	pub worldwide_unique_no: String,
	pub type_of_report: String,
	pub sender: String,
	pub manufacturer: String,
	pub wf_role: String,
	pub wf_user: String,
	pub receiver: String,
	pub creation_type: String,
	pub reviewed: bool,
	pub locked: bool,
	pub deleted: bool,
}

// -- CaseBmc (Business Model Controller)

pub struct CaseBmc;

impl DbBmc for CaseBmc {
	const TABLE: &'static str = "cases";
}

impl CaseBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		case_c: CaseForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, case_c).await
	}

	pub async fn get(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<Case> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<CaseFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<Case>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		case_u: CaseForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, case_u).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	/// List cases as link-option projections (id, report id, version, transmission date).
	/// Must be called from inside an RLS-scoped read context (e.g. `with_rls_read`).
	pub async fn list_link_options(dbx: &Dbx) -> Result<Vec<CaseLinkOption>> {
		dbx.fetch_all(sqlx::query_as::<_, CaseLinkOption>(
			"SELECT c.id AS case_id,
			        c.safety_report_id,
			        c.version,
			        s.transmission_date,
			        c.created_at
			   FROM cases c
			   LEFT JOIN safety_report_identification s ON s.case_id = c.id
			  ORDER BY c.created_at DESC
			  LIMIT 200",
		))
		.await
		.map_err(crate::model::Error::from)
	}

	/// List cases using the reference QVIS grid projection.
	/// Must be called from inside an RLS-scoped read context.
	pub async fn list_view_rows(
		dbx: &Dbx,
		list_options: Option<&ListOptions>,
	) -> Result<Vec<CaseListViewRow>> {
		let order_clause = list_view_order_clause(
			list_options.and_then(|options| options.order_bys.as_ref()),
		);
		let sql = format!(
			r#"
			SELECT row_number() OVER (ORDER BY {order_clause})::bigint AS no,
			       c.id AS case_id,
			       c.safety_report_id AS case_no,
			       GREATEST(c.version - 1, 0) AS fu,
			       COALESCE(s.transmission_date::text, c.created_at::date::text) AS date_of_creation,
			       COALESCE(NULLIF(c.dg_prd_key, ''), 'N/A') AS dg_prd_key,
			       '0' AS warn,
			       COALESCE(NULLIF(c.workflow_status, ''), c.status) AS wf_status,
			       COALESCE((
			       	SELECT cs.status
			       	  FROM case_submissions cs
			       	 WHERE cs.case_id = c.id
			       	 ORDER BY cs.submitted_at DESC
			       	 LIMIT 1
			       ), 'No') AS submission,
			       CASE
			       	WHEN EXISTS (
			       		SELECT 1
			       		  FROM reactions r
			       		 WHERE r.case_id = c.id
			       		   AND COALESCE(r.serious, false) = true
			       	)
			       	THEN 'Yes'
			       	ELSE 'No'
			       END AS sae,
			       COALESCE((
			       	SELECT NULLIF(r.reaction_meddra_code, '')
			       	  FROM reactions r
			       	 WHERE r.case_id = c.id
			       	 ORDER BY r.sequence_number ASC, r.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS meddra,
			       COALESCE((
			       	SELECT NULLIF(r.primary_source_reaction, '')
			       	  FROM reactions r
			       	 WHERE r.case_id = c.id
			       	 ORDER BY r.sequence_number ASC, r.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS ae_term,
			       COALESCE((
			       	SELECT NULLIF(si.sponsor_study_number, '')
			       	  FROM study_information si
			       	 WHERE si.case_id = c.id
			       	 ORDER BY si.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS study_no,
			       COALESCE((
			       	SELECT NULLIF(p.patient_initials, '')
			       	  FROM patient_information p
			       	 WHERE p.case_id = c.id
			       	 ORDER BY p.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS subject,
			       COALESCE(NULLIF(s.worldwide_unique_id, ''), 'N/A') AS worldwide_unique_no,
			       CASE s.report_type
			       	WHEN '1' THEN 'Spontaneous report'
			       	WHEN '2' THEN 'Report from study'
			       	WHEN '3' THEN 'Other'
			       	WHEN '4' THEN 'Not available to sender'
			       	ELSE COALESCE(NULLIF(s.report_type, ''), 'N/A')
			       END AS type_of_report,
			       COALESCE((
			       	SELECT NULLIF(sender.organization_name, '')
			       	  FROM sender_information sender
			       	 WHERE sender.case_id = c.id
			       	 ORDER BY sender.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS sender,
			       COALESCE((
			       	SELECT NULLIF(d.manufacturer_name, '')
			       	  FROM drug_information d
			       	 WHERE d.case_id = c.id
			       	 ORDER BY d.sequence_number ASC, d.created_at ASC
			       	 LIMIT 1
			       ), 'N/A') AS manufacturer,
			       COALESCE(NULLIF(c.workflow_assigned_role, ''), 'ALL') AS wf_role,
			       COALESCE((
			       	SELECT NULLIF(u.email, '')
			       	  FROM users u
			       	 WHERE u.id = c.workflow_assigned_user_id
			       	 LIMIT 1
			       ), 'ALL') AS wf_user,
			       COALESCE((
			       	SELECT NULLIF(ri.organization_name, '')
			       	  FROM receiver_information ri
			       	 WHERE ri.case_id = c.id
			       	 LIMIT 1
			       ), NULLIF(s.receiver_organization, ''), 'N/A') AS receiver,
			       CASE WHEN c.raw_xml IS NULL THEN 'Manual' ELSE 'Import' END AS creation_type,
			       c.status = 'reviewed' AS reviewed,
			       c.status = 'locked' AS locked,
			       c.status = 'deleted' AS deleted
			  FROM cases c
			  LEFT JOIN safety_report_identification s ON s.case_id = c.id
			 ORDER BY {order_clause}
			"#
		);

		dbx.fetch_all(sqlx::query_as::<_, CaseListViewRow>(&sql))
			.await
			.map_err(crate::model::Error::from)
	}
}

// -- CaseWorkflowEventRow (read projection)

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CaseWorkflowEventRow {
	pub id: Uuid,
	pub case_id: Uuid,
	pub from_status: String,
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub due_at: Option<OffsetDateTime>,
	pub acted_by: Uuid,
	pub actor_role_id: String,
	pub used_admin_override: bool,
	pub override_reason: Option<String>,
	pub created_at: OffsetDateTime,
}

// -- CaseWorkflowEvent types

#[derive(Debug)]
pub struct WorkflowTransitionRecord {
	pub case_id: Uuid,
	pub from_status: String,
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub due_at: Option<OffsetDateTime>,
	pub workflow_description: Option<String>,
	pub actor_user_id: Uuid,
	pub actor_role: String,
	pub used_admin_override: bool,
	pub override_reason: Option<String>,
}

#[derive(Debug)]
pub struct WorkflowAssignRecord {
	pub case_id: Uuid,
	pub current_status: String,
	pub target_role: String,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub due_at: Option<OffsetDateTime>,
	pub workflow_description: Option<String>,
	pub actor_user_id: Uuid,
	pub actor_role: String,
	pub used_admin_override: bool,
	pub override_reason: Option<String>,
}

// -- CaseWorkflowEventBmc

pub struct CaseWorkflowEventBmc;

impl CaseWorkflowEventBmc {
	/// Atomically update the case workflow status and insert an audit event.
	pub async fn record_transition(
		mm: &ModelManager,
		actor_user_id: Uuid,
		r: WorkflowTransitionRecord,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					UPDATE cases
					SET workflow_status = $2,
					    workflow_assigned_role = $3,
					    workflow_assigned_user_id = $4,
					    workflow_due_at = $5,
					    workflow_description = $6,
					    workflow_updated_at = now(),
					    updated_at = now(),
					    updated_by = $7
					WHERE id = $1
					"#,
				)
				.bind(r.case_id)
				.bind(&r.to_status)
				.bind(r.target_role.as_deref())
				.bind(r.target_user_id)
				.bind(r.due_at)
				.bind(r.workflow_description.as_deref())
				.bind(actor_user_id),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO case_workflow_events (
						case_id, from_status, to_status, target_role, target_user_id,
						comment, due_at, acted_by, actor_role_id, used_admin_override,
						override_reason
					) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
					"#,
				)
				.bind(r.case_id)
				.bind(&r.from_status)
				.bind(&r.to_status)
				.bind(r.target_role.as_deref())
				.bind(r.target_user_id)
				.bind(r.comment.as_deref())
				.bind(r.due_at)
				.bind(r.actor_user_id)
				.bind(canonical_role(&r.actor_role))
				.bind(r.used_admin_override)
				.bind(r.override_reason.as_deref()),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		Ok(())
	}

	/// Atomically update the case workflow assignment and insert an audit event.
	pub async fn record_assignment(
		mm: &ModelManager,
		actor_user_id: Uuid,
		r: WorkflowAssignRecord,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					UPDATE cases
					SET workflow_assigned_role = $2,
					    workflow_assigned_user_id = $3,
					    workflow_due_at = $4,
					    workflow_description = $5,
					    workflow_updated_at = now(),
					    updated_at = now(),
					    updated_by = $6
					WHERE id = $1
					"#,
				)
				.bind(r.case_id)
				.bind(&r.target_role)
				.bind(r.target_user_id)
				.bind(r.due_at)
				.bind(r.workflow_description.as_deref())
				.bind(actor_user_id),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO case_workflow_events (
						case_id, from_status, to_status, target_role, target_user_id,
						comment, due_at, acted_by, actor_role_id, used_admin_override,
						override_reason
					) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
					"#,
				)
				.bind(r.case_id)
				.bind(&r.current_status)
				.bind(&r.current_status)
				.bind(&r.target_role)
				.bind(r.target_user_id)
				.bind(r.comment.as_deref())
				.bind(r.due_at)
				.bind(r.actor_user_id)
				.bind(canonical_role(&r.actor_role))
				.bind(r.used_admin_override)
				.bind(r.override_reason.as_deref()),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		Ok(())
	}

	/// List all workflow events for a case, newest first.
	pub async fn list_by_case(
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<CaseWorkflowEventRow>> {
		let rows = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, CaseWorkflowEventRow>(
					r#"
					SELECT
						id, case_id, from_status, to_status, target_role, target_user_id,
						comment, due_at, acted_by, actor_role_id, used_admin_override,
						override_reason, created_at
					FROM case_workflow_events
					WHERE case_id = $1
					ORDER BY created_at DESC
					"#,
				)
				.bind(case_id),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;
		Ok(rows)
	}
}
