use crate::ctx::{canonical_role, Ctx};
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::store::dbx::Dbx;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{
	FilterNodes, ListOptions, OpValString, OpValValue, OpValsString, OpValsValue,
	OrderBy, OrderBys,
};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Case {
	pub id: Uuid,
	pub organization_id: Uuid,

	// E2B fields
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
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,

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
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
}

#[derive(Fields, Deserialize, Default)]
pub struct CaseForUpdate {
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
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

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SourceDocument {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub sequence_number: i32,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct SourceDocumentForCreate {
	pub case_id: Uuid,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub sequence_number: i32,
}

#[derive(Fields, Deserialize)]
pub struct SourceDocumentForUpdate {
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub sequence_number: Option<i32>,
}

#[derive(FilterNodes, Default)]
pub struct SourceDocumentFilter {
	pub case_id: Option<OpValsValue>,
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
				"s.safety_report_id ASC, c.id ASC"
			}
			"date_of_creation" | "dateOfCreation" => {
				"COALESCE(s.transmission_date, to_char(c.created_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS')) ASC, c.id ASC"
			}
			"dg_prd_key" | "dgPrdKey" => "c.dg_prd_key ASC NULLS LAST, c.id ASC",
			_ => "c.created_at DESC, c.id DESC",
		},
		OrderBy::Desc(field) => match field.as_str() {
			"created_at" => "c.created_at DESC, c.id DESC",
			"case_no" | "caseNo" | "safety_report_id" => {
				"s.safety_report_id DESC, c.id DESC"
			}
			"date_of_creation" | "dateOfCreation" => {
				"COALESCE(s.transmission_date, to_char(c.created_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS')) DESC, c.id DESC"
			}
			"dg_prd_key" | "dgPrdKey" => "c.dg_prd_key DESC NULLS LAST, c.id DESC",
			_ => "c.created_at DESC, c.id DESC",
		},
	}
}

fn list_view_rows_sql(order_clause: &str, where_clause: &str) -> String {
	format!(
		r#"
		SELECT row_number() OVER (ORDER BY {order_clause})::bigint AS no,
		       c.id AS case_id,
		       COALESCE(NULLIF(s.safety_report_id, ''), c.id::text) AS case_no,
		       GREATEST(COALESCE(s.version, 1) - 1, 0) AS fu,
		       COALESCE(s.transmission_date, to_char(c.created_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS')) AS date_of_creation,
		       COALESCE(s.date_of_most_recent_information::text, 'N/A') AS date_of_most_recent_information,
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
		       c.status = 'deleted' AS deleted,
		       (
		       	c.status IN ('validated', 'reviewed', 'locked')
		       	OR (
		       		c.raw_xml IS NOT NULL
		       		AND COALESCE(c.dirty_c, false) = false
		       		AND COALESCE(c.dirty_d, false) = false
		       		AND COALESCE(c.dirty_e, false) = false
		       		AND COALESCE(c.dirty_f, false) = false
		       		AND COALESCE(c.dirty_g, false) = false
		       		AND COALESCE(c.dirty_h, false) = false
		       	)
		       ) AS export_eligible
		  FROM cases c
		  LEFT JOIN safety_report_identification s ON s.case_id = c.id
		 {where_clause}
		 ORDER BY {order_clause}
		"#
	)
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
	case_u.dg_prd_key.is_some()
		|| case_u.review_receivers_json.is_some()
		|| case_u.workflow_routes_json.is_some()
		|| case_u.mfds_report_type.is_some()
		|| case_u.fda_report_type.is_some()
		|| case_u.report_year.is_some()
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
	pub transmission_date: Option<String>,
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
	pub date_of_most_recent_information: String,
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
	pub export_eligible: bool,
}

// -- CaseBmc (Business Model Controller)

pub struct CaseBmc;

impl DbBmc for CaseBmc {
	const TABLE: &'static str = "cases";
}

const CASE_SELECT: &str = r#"
	SELECT
		c.id,
		c.organization_id,
		c.dg_prd_key,
		c.status,
		c.review_receivers_json,
		c.workflow_routes_json,
		c.workflow_status,
		c.workflow_assigned_role,
		c.workflow_assigned_user_id,
		c.workflow_due_at,
		c.workflow_description,
		c.workflow_updated_at,
		c.mfds_report_type,
		c.fda_report_type,
		c.report_year,
		c.created_by,
		c.updated_by,
		c.submitted_by,
		c.submitted_at,
		c.raw_xml,
		c.dirty_c,
		c.dirty_d,
		c.dirty_e,
		c.dirty_f,
		c.dirty_g,
		c.dirty_h,
		c.created_at,
		c.updated_at
	FROM cases c
	LEFT JOIN safety_report_identification s ON s.case_id = c.id
"#;

fn first_string_eq(values: &OpValsString) -> Option<String> {
	values.0.iter().find_map(|op| match op {
		OpValString::Eq(value) => Some(value.clone()),
		_ => None,
	})
}

fn first_uuid_eq(values: &OpValsValue) -> Option<Uuid> {
	values.0.iter().find_map(|op| match op {
		OpValValue::Eq(value) => {
			value.as_str().and_then(|value| Uuid::parse_str(value).ok())
		}
		_ => None,
	})
}

impl CaseBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		case_c: CaseForCreate,
	) -> Result<Uuid> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let (id,) = dbx
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(
					"INSERT INTO cases (
						organization_id,
						dg_prd_key,
						status,
						review_receivers_json,
						workflow_routes_json,
						mfds_report_type,
						fda_report_type,
						report_year,
						created_by,
						updated_by,
						created_at,
						updated_at
					)
					VALUES ($1, $2, COALESCE($3, 'draft'), $4, $5, $6, $7, $8, $9, $9, now(), now())
					RETURNING id",
				)
				.bind(case_c.organization_id)
				.bind(case_c.dg_prd_key)
				.bind(case_c.status)
				.bind(case_c.review_receivers_json)
				.bind(case_c.workflow_routes_json)
				.bind(case_c.mfds_report_type)
				.bind(case_c.fda_report_type)
				.bind(case_c.report_year)
				.bind(ctx.user_id()),
			)
			.await?;
		dbx.commit_txn().await?;
		Ok(id)
	}

	pub async fn get(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<Case> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let sql = format!("{CASE_SELECT} WHERE c.id = $1");
		let entity = dbx
			.fetch_optional(sqlx::query_as::<_, Case>(&sql).bind(id))
			.await?;
		match entity {
			Some(entity) => {
				dbx.commit_txn().await?;
				Ok(entity)
			}
			None => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				})
			}
		}
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<CaseFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<Case>> {
		let mut conditions: Vec<String> = Vec::new();
		let mut organization_id: Option<Uuid> = None;
		let mut safety_report_id: Option<String> = None;
		let mut status: Option<String> = None;
		if let Some(filters) = filters {
			for filter in filters {
				if organization_id.is_none() {
					organization_id =
						filter.organization_id.as_ref().and_then(first_uuid_eq);
				}
				if safety_report_id.is_none() {
					safety_report_id =
						filter.safety_report_id.as_ref().and_then(first_string_eq);
				}
				if status.is_none() {
					status = filter.status.as_ref().and_then(first_string_eq);
				}
			}
		}
		if organization_id.is_some() {
			conditions
				.push(format!("c.organization_id = ${}", conditions.len() + 1));
		}
		if safety_report_id.is_some() {
			conditions
				.push(format!("s.safety_report_id = ${}", conditions.len() + 1));
		}
		if status.is_some() {
			conditions.push(format!("c.status = ${}", conditions.len() + 1));
		}
		let mut sql = CASE_SELECT.to_string();
		if !conditions.is_empty() {
			sql.push_str(" WHERE ");
			sql.push_str(&conditions.join(" AND "));
		}
		sql.push_str(" ORDER BY c.created_at DESC, c.id DESC");
		if let Some(limit) = list_options.as_ref().and_then(|options| options.limit)
		{
			sql.push_str(&format!(" LIMIT {}", limit.clamp(0, 5000)));
		} else {
			sql.push_str(" LIMIT 1000");
		}
		if let Some(offset) =
			list_options.as_ref().and_then(|options| options.offset)
		{
			sql.push_str(&format!(" OFFSET {}", offset.max(0)));
		}

		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let mut query = sqlx::query_as::<_, Case>(&sql);
		if let Some(value) = organization_id {
			query = query.bind(value);
		}
		if let Some(value) = safety_report_id {
			query = query.bind(value);
		}
		if let Some(value) = status {
			query = query.bind(value);
		}
		let entities = dbx.fetch_all(query).await?;
		dbx.commit_txn().await?;
		Ok(entities)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		case_u: CaseForUpdate,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let count = dbx
			.execute(
				sqlx::query(
					"UPDATE cases
					 SET dg_prd_key = COALESCE($2, dg_prd_key),
					     status = COALESCE($3, status),
					     review_receivers_json = COALESCE($4, review_receivers_json),
					     workflow_routes_json = COALESCE($5, workflow_routes_json),
					     mfds_report_type = COALESCE($6, mfds_report_type),
					     fda_report_type = COALESCE($7, fda_report_type),
					     report_year = COALESCE($8, report_year),
					     submitted_by = COALESCE($9, submitted_by),
					     submitted_at = COALESCE($10, submitted_at),
					     raw_xml = COALESCE($11, raw_xml),
					     dirty_c = COALESCE($12, dirty_c),
					     dirty_d = COALESCE($13, dirty_d),
					     dirty_e = COALESCE($14, dirty_e),
					     dirty_f = COALESCE($15, dirty_f),
					     dirty_g = COALESCE($16, dirty_g),
					     dirty_h = COALESCE($17, dirty_h),
					     updated_at = now(),
					     updated_by = $18
					 WHERE id = $1",
				)
				.bind(id)
				.bind(case_u.dg_prd_key)
				.bind(case_u.status)
				.bind(case_u.review_receivers_json)
				.bind(case_u.workflow_routes_json)
				.bind(case_u.mfds_report_type)
				.bind(case_u.fda_report_type)
				.bind(case_u.report_year)
				.bind(case_u.submitted_by)
				.bind(case_u.submitted_at)
				.bind(case_u.raw_xml)
				.bind(case_u.dirty_c)
				.bind(case_u.dirty_d)
				.bind(case_u.dirty_e)
				.bind(case_u.dirty_f)
				.bind(case_u.dirty_g)
				.bind(case_u.dirty_h)
				.bind(ctx.user_id()),
			)
			.await?;
		if count == 0 {
			dbx.rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		dbx.commit_txn().await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	/// List cases as link-option projections (id, report id, version, transmission date).
	/// Must be called from inside an RLS-scoped read context (e.g. `with_rls_read`).
	pub async fn list_link_options(dbx: &Dbx) -> Result<Vec<CaseLinkOption>> {
		dbx.fetch_all(sqlx::query_as::<_, CaseLinkOption>(
			"SELECT c.id AS case_id,
			        s.safety_report_id,
			        s.version,
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
		let sql = list_view_rows_sql(order_clause, "");

		dbx.fetch_all(sqlx::query_as::<_, CaseListViewRow>(&sql))
			.await
			.map_err(crate::model::Error::from)
	}

	/// List case grid projections for a known case-id set.
	/// Must be called from inside an RLS-scoped read context.
	pub async fn list_view_rows_by_ids(
		dbx: &Dbx,
		case_ids: &[Uuid],
	) -> Result<Vec<CaseListViewRow>> {
		if case_ids.is_empty() {
			return Ok(Vec::new());
		}
		let sql = list_view_rows_sql(
			"c.created_at DESC, c.id DESC",
			"WHERE c.id = ANY($1)",
		);
		dbx.fetch_all(sqlx::query_as::<_, CaseListViewRow>(&sql).bind(case_ids))
			.await
			.map_err(crate::model::Error::from)
	}
}

pub struct SourceDocumentBmc;
impl DbBmc for SourceDocumentBmc {
	const TABLE: &'static str = "source_documents";
}

impl SourceDocumentBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: SourceDocumentForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<SourceDocument> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<SourceDocumentFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<SourceDocument>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: SourceDocumentForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

// -- CaseWorkflowEventRow (read projection)

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CaseWorkflowEventRow {
	pub id: Uuid,
	pub case_id: Uuid,
	pub from_status: String,
	pub from_role: Option<String>,
	pub from_user_id: Option<Uuid>,
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub date_of_most_recent: Option<String>,
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
	pub from_role: Option<String>,
	pub from_user_id: Option<Uuid>,
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub date_of_most_recent: Option<String>,
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
	pub from_role: Option<String>,
	pub from_user_id: Option<Uuid>,
	pub target_role: String,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub date_of_most_recent: Option<String>,
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
		ctx: &Ctx,
		mm: &ModelManager,
		actor_user_id: Uuid,
		r: WorkflowTransitionRecord,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

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
						case_id, from_status, from_role, from_user_id, to_status,
						target_role, target_user_id, comment, date_of_most_recent,
						due_at, acted_by, actor_role_id, used_admin_override,
						override_reason
					) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
					"#,
				)
				.bind(r.case_id)
				.bind(&r.from_status)
				.bind(r.from_role.as_deref())
				.bind(r.from_user_id)
				.bind(&r.to_status)
				.bind(r.target_role.as_deref())
				.bind(r.target_user_id)
				.bind(r.comment.as_deref())
				.bind(r.date_of_most_recent.as_deref())
				.bind(r.due_at)
				.bind(r.actor_user_id)
				.bind(canonical_role(&r.actor_role))
				.bind(r.used_admin_override)
				.bind(r.override_reason.as_deref()),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		mm.dbx().commit_txn().await?;
		Ok(())
	}

	/// Atomically update the case workflow assignment and insert an audit event.
	pub async fn record_assignment(
		ctx: &Ctx,
		mm: &ModelManager,
		actor_user_id: Uuid,
		r: WorkflowAssignRecord,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

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
						case_id, from_status, from_role, from_user_id, to_status,
						target_role, target_user_id, comment, date_of_most_recent,
						due_at, acted_by, actor_role_id, used_admin_override,
						override_reason
					) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
					"#,
				)
				.bind(r.case_id)
				.bind(&r.current_status)
				.bind(r.from_role.as_deref())
				.bind(r.from_user_id)
				.bind(&r.current_status)
				.bind(&r.target_role)
				.bind(r.target_user_id)
				.bind(r.comment.as_deref())
				.bind(r.date_of_most_recent.as_deref())
				.bind(r.due_at)
				.bind(r.actor_user_id)
				.bind(canonical_role(&r.actor_role))
				.bind(r.used_admin_override)
				.bind(r.override_reason.as_deref()),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;

		mm.dbx().commit_txn().await?;
		Ok(())
	}

	/// List all workflow events for a case, newest first.
	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<CaseWorkflowEventRow>> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let rows = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, CaseWorkflowEventRow>(
					r#"
					SELECT
						id, case_id, from_status, from_role, from_user_id, to_status,
						target_role, target_user_id, comment, date_of_most_recent,
						due_at, acted_by, actor_role_id, used_admin_override,
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
		mm.dbx().commit_txn().await?;
		Ok(rows)
	}
}
