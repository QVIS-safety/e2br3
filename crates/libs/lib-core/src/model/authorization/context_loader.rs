use crate::authorization::{
	AuditLogResource, BuiltInIdentityKind, CaseAuditTrailResource,
	CaseChildResource, CaseCreateProposal, CaseResource, Collection,
	ContextSnapshot, EnforcedScopeFilter, EvaluatedContext, Existing,
	ImportHistoryResource, LockedMutationContext, NoticeResource, Parent,
	PresaveCreateProposal, PresaveResource, Proposed, RequestAuthorizationSnapshot,
	ResourceSet, SettingsResource, SubmissionResource, TerminologyImportProposal,
	TerminologyResource, UserResource, XmlImportBatchProposal,
};
use crate::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use crate::model::store::dbx::Dbx;
use sqlx::FromRow;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug)]
pub enum AuthorizationFactLoadError {
	Database(crate::model::store::dbx::Error),
	FactNotFound {
		fact: &'static str,
		id: Uuid,
	},
	SnapshotStale {
		snapshot_organization_revision: i64,
		current_organization_revision: i64,
		snapshot_principal_revision: i64,
		current_principal_revision: i64,
	},
}

impl Display for AuthorizationFactLoadError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for AuthorizationFactLoadError {}

impl From<crate::model::store::dbx::Error> for AuthorizationFactLoadError {
	fn from(error: crate::model::store::dbx::Error) -> Self {
		Self::Database(error)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMutationKind {
	Edit,
	ReviewToggle,
	LockToggle,
	Delete,
	Restore,
	Submission,
	ChildEdit,
	WorkflowTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresaveAuthorizationKind {
	Sender,
	Product,
	Study,
	Receiver,
	Reporter,
	Narrative,
}

impl PresaveAuthorizationKind {
	fn table(self) -> &'static str {
		match self {
			Self::Sender => "sender_presaves",
			Self::Product => "product_presaves",
			Self::Study => "study_presaves",
			Self::Receiver => "receiver_presaves",
			Self::Reporter => "reporter_presaves",
			Self::Narrative => "narrative_presaves",
		}
	}

	fn fingerprint(self) -> &'static str {
		match self {
			Self::Sender => "sender",
			Self::Product => "product",
			Self::Study => "study",
			Self::Receiver => "receiver",
			Self::Reporter => "reporter",
			Self::Narrative => "narrative",
		}
	}
}

#[derive(Debug, FromRow)]
struct PresaveFacts {
	organization_id: Uuid,
	product_presave_id: Option<Uuid>,
	sender_presave_id: Option<Uuid>,
	child_product_ids: Vec<Uuid>,
	child_study_ids: Vec<Uuid>,
}

#[derive(Debug, FromRow)]
struct CaseFacts {
	organization_id: Uuid,
	status: String,
	sender_identifiers: Vec<String>,
	product_identifiers: Vec<String>,
	study_identifiers: Vec<String>,
}

#[derive(Debug, FromRow)]
struct SubmissionParent {
	case_id: Uuid,
}

#[derive(Debug, FromRow)]
struct ImportHistoryFacts {
	organization_id: Uuid,
	case_id: Option<Uuid>,
	uploaded_by: Uuid,
}

#[derive(Debug, FromRow)]
struct ExportHistoryParent {
	case_id: Uuid,
}

#[derive(Debug, FromRow)]
struct UserMutationFacts {
	organization_id: Uuid,
	protected_administrator: bool,
}

pub struct AuthorizationFactLoader<'tx> {
	dbx: &'tx Dbx,
	snapshot: &'tx RequestAuthorizationSnapshot,
}

impl<'tx> AuthorizationFactLoader<'tx> {
	pub fn new(dbx: &'tx Dbx, snapshot: &'tx RequestAuthorizationSnapshot) -> Self {
		Self { dbx, snapshot }
	}

	pub fn case_collection(&self) -> ContextSnapshot<'tx, Collection<CaseResource>> {
		ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!("cases:{}", self.snapshot.organization_id()),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		})
	}

	pub fn presave_collection(
		&self,
	) -> ContextSnapshot<'tx, Collection<PresaveResource>> {
		ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"presaves:{}",
				self.snapshot.organization_id()
			),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		})
	}

	pub async fn user_for_mutation(
		&self,
		user_id: Uuid,
	) -> Result<
		(LockedMutationContext<'tx, Existing<UserResource>>, bool),
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		let facts = self
			.dbx
			.fetch_optional(
				sqlx::query_as::<_, UserMutationFacts>(
					r#"
					SELECT organization_id,
					       role IN ($2, $3, $4) AS protected_administrator
					  FROM users
					 WHERE id = $1
					 FOR UPDATE
					"#,
				)
				.bind(user_id)
				.bind(ROLE_SYSTEM_ADMIN)
				.bind(ROLE_SPONSOR_ADMIN_CRO)
				.bind(ROLE_SPONSOR_ADMIN_COMPANY),
			)
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "user",
				id: user_id,
			})?;
		Ok((
			LockedMutationContext::new(EvaluatedContext {
				organization_id: Some(facts.organization_id),
				target_fingerprint: format!("user:{user_id}"),
				within_principal_scope: false,
				lifecycle_compatible: false,
				parent_authorized: false,
				every_target_authorized: false,
				enforced_scope_filter: None,
			}),
			facts.protected_administrator,
		))
	}

	pub async fn presave_create_for_mutation(
		&self,
		fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Proposed<PresaveCreateProposal>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		Ok(LockedMutationContext::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"presave-create:{}:{}",
				self.snapshot.organization_id(),
				fingerprint.as_ref()
			),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		}))
	}

	pub async fn presave_existing(
		&self,
		kind: PresaveAuthorizationKind,
		id: Uuid,
	) -> Result<
		ContextSnapshot<'tx, Existing<PresaveResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_presave_facts(kind, id, false).await?;
		Ok(ContextSnapshot::new(presave_evaluated(
			self.snapshot,
			kind,
			id,
			facts.organization_id,
			facts.product_presave_id,
			facts.sender_presave_id,
			&facts.child_product_ids,
			&facts.child_study_ids,
		)))
	}

	pub async fn presave_for_mutation(
		&self,
		kind: PresaveAuthorizationKind,
		id: Uuid,
	) -> Result<
		LockedMutationContext<'tx, Existing<PresaveResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		let facts = self.load_presave_facts(kind, id, true).await?;
		Ok(LockedMutationContext::new(presave_evaluated(
			self.snapshot,
			kind,
			id,
			facts.organization_id,
			facts.product_presave_id,
			facts.sender_presave_id,
			&facts.child_product_ids,
			&facts.child_study_ids,
		)))
	}

	async fn load_presave_facts(
		&self,
		kind: PresaveAuthorizationKind,
		id: Uuid,
		for_update: bool,
	) -> Result<PresaveFacts, AuthorizationFactLoadError> {
		let lock = if for_update { " FOR UPDATE" } else { "" };
		let sql = match kind {
			PresaveAuthorizationKind::Sender => format!(
				r#"
				SELECT sender.organization_id,
				       NULL::uuid AS product_presave_id,
				       NULL::uuid AS sender_presave_id,
				       COALESCE(
				        (SELECT array_agg(DISTINCT product.id)
				           FROM product_presaves product
				          WHERE product.sender_presave_id = sender.id
				            AND product.deleted = false),
				        ARRAY[]::uuid[]
				       ) AS child_product_ids,
				       COALESCE(
				        (SELECT array_agg(DISTINCT study.id)
				           FROM study_presaves study
				           JOIN product_presaves product
				             ON product.id = study.product_presave_id
				            AND product.sender_presave_id = sender.id
				          WHERE study.deleted = false
				            AND product.deleted = false),
				        ARRAY[]::uuid[]
				       ) AS child_study_ids
				  FROM sender_presaves sender
				 WHERE sender.id = $1
				 {lock}
				"#
			),
			PresaveAuthorizationKind::Product => format!(
				r#"
				SELECT product.organization_id,
				       NULL::uuid AS product_presave_id,
				       product.sender_presave_id,
				       ARRAY[]::uuid[] AS child_product_ids,
				       COALESCE(
				        (SELECT array_agg(DISTINCT study.id)
				           FROM study_presaves study
				          WHERE study.product_presave_id = product.id
				            AND study.deleted = false),
				        ARRAY[]::uuid[]
				       ) AS child_study_ids
				  FROM product_presaves product
				 WHERE product.id = $1
				 {lock}
				"#
			),
			PresaveAuthorizationKind::Study => format!(
				r#"
				SELECT study.organization_id,
				       study.product_presave_id,
				       (SELECT product.sender_presave_id
				          FROM product_presaves product
				         WHERE product.id = study.product_presave_id
				           AND product.deleted = false) AS sender_presave_id,
				       ARRAY[]::uuid[] AS child_product_ids,
				       ARRAY[]::uuid[] AS child_study_ids
				  FROM study_presaves study
				 WHERE study.id = $1{lock}
				"#
			),
			_ => format!(
				"SELECT organization_id, NULL::uuid AS product_presave_id, NULL::uuid AS sender_presave_id, ARRAY[]::uuid[] AS child_product_ids, ARRAY[]::uuid[] AS child_study_ids FROM {} WHERE id = $1{lock}",
				kind.table()
			),
		};
		self.dbx
			.fetch_optional(sqlx::query_as::<_, PresaveFacts>(&sql).bind(id))
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "presave",
				id,
			})
	}

	pub fn settings_existing(
		&self,
	) -> ContextSnapshot<'tx, Existing<SettingsResource>> {
		ContextSnapshot::new(organization_resource_evaluated(
			self.snapshot.organization_id(),
			"settings",
		))
	}

	pub async fn settings_for_mutation(
		&self,
	) -> Result<
		LockedMutationContext<'tx, Existing<SettingsResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		Ok(LockedMutationContext::new(organization_resource_evaluated(
			self.snapshot.organization_id(),
			"settings",
		)))
	}

	pub async fn notice_for_mutation(
		&self,
	) -> Result<
		LockedMutationContext<'tx, Existing<NoticeResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		Ok(LockedMutationContext::new(organization_resource_evaluated(
			self.snapshot.organization_id(),
			"notice",
		)))
	}

	pub fn audit_log_collection(
		&self,
	) -> ContextSnapshot<'tx, Collection<AuditLogResource>> {
		ContextSnapshot::new(organization_resource_evaluated(
			self.snapshot.organization_id(),
			"audit-logs",
		))
	}

	pub fn terminology_collection(
		&self,
	) -> ContextSnapshot<'tx, Collection<TerminologyResource>> {
		ContextSnapshot::new(organization_resource_evaluated(
			self.snapshot.organization_id(),
			"terminology",
		))
	}

	pub async fn terminology_import_for_mutation(
		&self,
		fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Proposed<TerminologyImportProposal>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		Ok(LockedMutationContext::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"terminology:{}:{}",
				self.snapshot.organization_id(),
				fingerprint.as_ref()
			),
			within_principal_scope: false,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}))
	}

	pub async fn case_resource_set(
		&self,
		case_ids: &[Uuid],
	) -> Result<
		ContextSnapshot<'tx, ResourceSet<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		let mut ids = case_ids.to_vec();
		ids.sort_unstable();
		ids.dedup();
		let mut every_target_authorized = true;
		for case_id in &ids {
			let facts = self.load_case_facts(*case_id, false).await?;
			let same_organization = facts.organization_id
				== self.snapshot.organization_id()
				|| self.snapshot.identity().is_platform_administrator();
			if !same_organization || !case_within_scope(self.snapshot, &facts) {
				every_target_authorized = false;
			}
		}
		Ok(ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"case-export:{}",
				ids.iter()
					.map(Uuid::to_string)
					.collect::<Vec<_>>()
					.join(",")
			),
			within_principal_scope: false,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized,
			enforced_scope_filter: None,
		}))
	}

	pub async fn export_history_parent_case_id(
		&self,
		history_id: Uuid,
	) -> Result<Uuid, AuthorizationFactLoadError> {
		self.dbx
			.fetch_optional(
				sqlx::query_as::<_, ExportHistoryParent>(
					"SELECT case_id
					   FROM xml_export_history
					  WHERE id = $1",
				)
				.bind(history_id),
			)
			.await?
			.map(|row| row.case_id)
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "export_history",
				id: history_id,
			})
	}

	pub fn submission_collection(
		&self,
	) -> ContextSnapshot<'tx, Collection<SubmissionResource>> {
		ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"submissions:{}",
				self.snapshot.organization_id()
			),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		})
	}

	pub fn import_history_collection(
		&self,
	) -> ContextSnapshot<'tx, Collection<ImportHistoryResource>> {
		ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"import-history:{}",
				self.snapshot.organization_id()
			),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		})
	}

	pub async fn import_history_existing(
		&self,
		history_id: Uuid,
	) -> Result<
		ContextSnapshot<'tx, Existing<ImportHistoryResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self
			.dbx
			.fetch_optional(
				sqlx::query_as::<_, ImportHistoryFacts>(
					"SELECT u.organization_id, h.case_id, h.uploaded_by
					   FROM xml_import_history h
					   JOIN users u ON u.id = h.uploaded_by
					  WHERE h.id = $1",
				)
				.bind(history_id),
			)
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "import_history",
				id: history_id,
			})?;
		let within_principal_scope = match facts.case_id {
			Some(case_id) => {
				let case = self.load_case_facts(case_id, false).await?;
				case_within_scope(self.snapshot, &case)
			}
			None => {
				facts.uploaded_by == self.snapshot.principal_id()
					|| matches!(
						self.snapshot.identity().built_in_kind(),
						Some(
							BuiltInIdentityKind::PlatformAdministrator
								| BuiltInIdentityKind::SponsorCroAdministrator
								| BuiltInIdentityKind::SponsorCompanyAdministrator
						)
					)
			}
		};
		Ok(ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(facts.organization_id),
			target_fingerprint: format!("import-history:{history_id}"),
			within_principal_scope,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}))
	}

	pub async fn xml_import_batch_for_mutation(
		&self,
		fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Proposed<XmlImportBatchProposal>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		Ok(LockedMutationContext::new(EvaluatedContext {
			organization_id: Some(self.snapshot.organization_id()),
			target_fingerprint: format!(
				"xml-import:{}:{}",
				self.snapshot.organization_id(),
				fingerprint.as_ref()
			),
			within_principal_scope: true,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: Some(scope_filter(self.snapshot)),
		}))
	}

	pub async fn submission_existing(
		&self,
		submission_id: Uuid,
	) -> Result<
		ContextSnapshot<'tx, Existing<SubmissionResource>>,
		AuthorizationFactLoadError,
	> {
		let parent = self
			.dbx
			.fetch_optional(
				sqlx::query_as::<_, SubmissionParent>(
					"SELECT cs.case_id
			   FROM case_submissions cs
			  WHERE cs.id = $1",
				)
				.bind(submission_id),
			)
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "submission",
				id: submission_id,
			})?;
		let case = self.load_case_facts(parent.case_id, false).await?;
		Ok(ContextSnapshot::new(EvaluatedContext {
			organization_id: Some(case.organization_id),
			target_fingerprint: format!(
				"submission:{submission_id}:case:{}",
				parent.case_id
			),
			within_principal_scope: case_within_scope(self.snapshot, &case),
			lifecycle_compatible: !matches!(
				case.status.trim().to_ascii_lowercase().as_str(),
				"deleted" | "archived"
			),
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}))
	}

	pub async fn submission_parent_case_for_mutation(
		&self,
		submission_id: Uuid,
	) -> Result<
		LockedMutationContext<'tx, Existing<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		let parent = self
			.dbx
			.fetch_optional(
				sqlx::query_as::<_, SubmissionParent>(
					"SELECT cs.case_id
					   FROM case_submissions cs
					  WHERE cs.id = $1
					  FOR UPDATE OF cs",
				)
				.bind(submission_id),
			)
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "submission",
				id: submission_id,
			})?;
		let case = self.load_case_facts(parent.case_id, true).await?;
		Ok(LockedMutationContext::new(EvaluatedContext {
			organization_id: Some(case.organization_id),
			target_fingerprint: format!(
				"case:{}:submission:{submission_id}",
				parent.case_id,
			),
			within_principal_scope: case_within_scope(self.snapshot, &case),
			lifecycle_compatible: !matches!(
				case.status.trim().to_ascii_lowercase().as_str(),
				"deleted" | "archived"
			),
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}))
	}

	pub async fn case_create_for_verified_mutation(
		&self,
		organization_id: Uuid,
		product_key: Option<&str>,
	) -> Result<
		LockedMutationContext<'tx, Proposed<CaseCreateProposal>>,
		AuthorizationFactLoadError,
	> {
		let (product_identifiers, sender_identifiers) = if let Some(product_key) =
			product_key.map(str::trim).filter(|value| !value.is_empty())
		{
			self.dbx
				.fetch_one(
					sqlx::query_as::<_, (Vec<String>, Vec<String>)>(
						"SELECT COALESCE(array_agg(id::text), ARRAY[]::text[]),
						        COALESCE(array_agg(sender_presave_id::text)
						                 FILTER (WHERE sender_presave_id IS NOT NULL), ARRAY[]::text[])
						   FROM product_presaves
						  WHERE organization_id = $1
						    AND product_id = $2
						    AND deleted = false",
					)
					.bind(organization_id)
					.bind(product_key),
				)
				.await?
		} else {
			(Vec::new(), Vec::new())
		};
		let within_principal_scope =
			self.snapshot.identity().is_platform_administrator()
				|| (organization_id == self.snapshot.organization_id()
					&& scope_allows(
						self.snapshot.scope().sender_ids(),
						&sender_identifiers,
					) && scope_allows(
					self.snapshot.scope().product_ids(),
					&product_identifiers,
				));
		Ok(LockedMutationContext::new(EvaluatedContext {
			organization_id: Some(organization_id),
			target_fingerprint: format!("case:new:{organization_id}"),
			within_principal_scope,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}))
	}

	pub async fn case_existing(
		&self,
		case_id: Uuid,
	) -> Result<
		ContextSnapshot<'tx, Existing<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, false).await?;
		Ok(ContextSnapshot::new(case_evaluated(
			self.snapshot,
			case_id,
			&facts,
			false,
		)))
	}

	pub async fn case_existing_for_mutation(
		&self,
		case_id: Uuid,
		kind: CaseMutationKind,
	) -> Result<
		LockedMutationContext<'tx, Existing<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		self.case_existing_for_verified_mutation(case_id, kind)
			.await
	}

	pub async fn case_existing_for_verified_mutation(
		&self,
		case_id: Uuid,
		kind: CaseMutationKind,
	) -> Result<
		LockedMutationContext<'tx, Existing<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, true).await?;
		Ok(LockedMutationContext::new(case_evaluated(
			self.snapshot,
			case_id,
			&facts,
			case_lifecycle_allows(&facts, kind),
		)))
	}

	pub async fn case_existing_scope_postcondition(
		&self,
		case_id: Uuid,
	) -> Result<
		LockedMutationContext<'tx, Existing<CaseResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, true).await?;
		Ok(LockedMutationContext::new(case_evaluated(
			self.snapshot,
			case_id,
			&facts,
			true,
		)))
	}

	pub async fn case_child(
		&self,
		case_id: Uuid,
		child_fingerprint: impl AsRef<str>,
	) -> Result<
		ContextSnapshot<'tx, Parent<CaseResource, CaseChildResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, false).await?;
		Ok(ContextSnapshot::new(case_child_evaluated(
			self.snapshot,
			case_id,
			child_fingerprint.as_ref(),
			&facts,
			false,
		)))
	}

	pub async fn case_audit_trail(
		&self,
		case_id: Uuid,
	) -> Result<
		ContextSnapshot<'tx, Parent<CaseResource, CaseAuditTrailResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, false).await?;
		Ok(ContextSnapshot::new(case_child_evaluated(
			self.snapshot,
			case_id,
			"audit-trail",
			&facts,
			false,
		)))
	}

	pub async fn case_child_for_mutation(
		&self,
		case_id: Uuid,
		child_fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Parent<CaseResource, CaseChildResource>>,
		AuthorizationFactLoadError,
	> {
		self.lock_and_verify_revisions().await?;
		self.case_child_for_verified_mutation(case_id, child_fingerprint)
			.await
	}

	pub async fn case_child_for_verified_mutation(
		&self,
		case_id: Uuid,
		child_fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Parent<CaseResource, CaseChildResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, true).await?;
		Ok(LockedMutationContext::new(case_child_evaluated(
			self.snapshot,
			case_id,
			child_fingerprint.as_ref(),
			&facts,
			case_lifecycle_allows(&facts, CaseMutationKind::ChildEdit),
		)))
	}

	pub async fn case_child_scope_postcondition(
		&self,
		case_id: Uuid,
		child_fingerprint: impl AsRef<str>,
	) -> Result<
		LockedMutationContext<'tx, Parent<CaseResource, CaseChildResource>>,
		AuthorizationFactLoadError,
	> {
		let facts = self.load_case_facts(case_id, true).await?;
		Ok(LockedMutationContext::new(case_child_evaluated(
			self.snapshot,
			case_id,
			child_fingerprint.as_ref(),
			&facts,
			true,
		)))
	}

	pub async fn lock_and_verify_revisions(
		&self,
	) -> Result<(), AuthorizationFactLoadError> {
		let (organization_revision, principal_revision) = self
			.dbx
			.fetch_one(
				sqlx::query_as::<_, (i64, i64)>(
					"SELECT organization_revision, principal_revision
					   FROM authz_lock_policy_revisions($1, $2)",
				)
				.bind(self.snapshot.principal_id())
				.bind(self.snapshot.organization_id()),
			)
			.await?;
		ensure_current_revisions(
			self.snapshot,
			organization_revision,
			principal_revision,
		)
	}

	async fn load_case_facts(
		&self,
		case_id: Uuid,
		for_update: bool,
	) -> Result<CaseFacts, AuthorizationFactLoadError> {
		let lock_clause = if for_update { "FOR UPDATE OF c" } else { "" };
		let sql = format!(
			r#"
			SELECT c.organization_id,
			       c.status,
			       COALESCE(
			        (SELECT array_agg(identifier)
			           FROM case_scope_identifiers(c.id)
			          WHERE scope_kind = 'sender'),
			        ARRAY[]::text[]
			       ) AS sender_identifiers,
			       COALESCE(
			        (SELECT array_agg(identifier)
			           FROM case_scope_identifiers(c.id)
			          WHERE scope_kind = 'product'),
			        ARRAY[]::text[]
			       ) AS product_identifiers,
			       COALESCE(
			        (SELECT array_agg(identifier)
			           FROM case_scope_identifiers(c.id)
			          WHERE scope_kind = 'study'),
			        ARRAY[]::text[]
			       ) AS study_identifiers
			  FROM cases c
			 WHERE c.id = $1
			 {lock_clause}
			"#
		);
		self.dbx
			.fetch_optional(sqlx::query_as::<_, CaseFacts>(&sql).bind(case_id))
			.await?
			.ok_or(AuthorizationFactLoadError::FactNotFound {
				fact: "case",
				id: case_id,
			})
	}
}

fn scope_filter(snapshot: &RequestAuthorizationSnapshot) -> EnforcedScopeFilter {
	EnforcedScopeFilter::new(
		snapshot.scope().sender_ids().to_vec(),
		snapshot.scope().product_ids().to_vec(),
		snapshot.scope().study_ids().to_vec(),
		snapshot.scope().blind_allowed(),
	)
}

fn organization_resource_evaluated(
	organization_id: Uuid,
	resource: &str,
) -> EvaluatedContext {
	EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("{resource}:{organization_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	}
}

fn case_evaluated(
	snapshot: &RequestAuthorizationSnapshot,
	case_id: Uuid,
	facts: &CaseFacts,
	lifecycle_compatible: bool,
) -> EvaluatedContext {
	EvaluatedContext {
		organization_id: Some(facts.organization_id),
		target_fingerprint: format!("case:{case_id}:{}", facts.status),
		within_principal_scope: case_within_scope(snapshot, facts),
		lifecycle_compatible,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	}
}

fn case_child_evaluated(
	snapshot: &RequestAuthorizationSnapshot,
	case_id: Uuid,
	child_fingerprint: &str,
	facts: &CaseFacts,
	lifecycle_compatible: bool,
) -> EvaluatedContext {
	let same_organization = facts.organization_id == snapshot.organization_id()
		|| snapshot.identity().is_platform_administrator();
	EvaluatedContext {
		organization_id: Some(facts.organization_id),
		target_fingerprint: format!("case:{case_id}:child:{child_fingerprint}"),
		within_principal_scope: false,
		lifecycle_compatible,
		parent_authorized: same_organization && case_within_scope(snapshot, facts),
		every_target_authorized: false,
		enforced_scope_filter: None,
	}
}

fn case_within_scope(
	snapshot: &RequestAuthorizationSnapshot,
	facts: &CaseFacts,
) -> bool {
	scope_allows(snapshot.scope().sender_ids(), &facts.sender_identifiers)
		&& scope_allows(snapshot.scope().product_ids(), &facts.product_identifiers)
		&& scope_allows(snapshot.scope().study_ids(), &facts.study_identifiers)
}

fn scope_allows(assigned: &[String], available: &[String]) -> bool {
	if assigned.is_empty() || available.is_empty() {
		return true;
	}
	available.iter().any(|candidate| {
		assigned
			.iter()
			.any(|assigned| assigned.eq_ignore_ascii_case(candidate))
	})
}

fn presave_evaluated(
	snapshot: &RequestAuthorizationSnapshot,
	kind: PresaveAuthorizationKind,
	id: Uuid,
	organization_id: Uuid,
	product_presave_id: Option<Uuid>,
	sender_presave_id: Option<Uuid>,
	child_product_ids: &[Uuid],
	child_study_ids: &[Uuid],
) -> EvaluatedContext {
	EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("presave:{}:{id}", kind.fingerprint()),
		within_principal_scope: presave_within_scope(
			snapshot,
			kind,
			id,
			product_presave_id,
			sender_presave_id,
			child_product_ids,
			child_study_ids,
		),
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	}
}

fn presave_within_scope(
	snapshot: &RequestAuthorizationSnapshot,
	kind: PresaveAuthorizationKind,
	id: Uuid,
	product_presave_id: Option<Uuid>,
	sender_presave_id: Option<Uuid>,
	child_product_ids: &[Uuid],
	child_study_ids: &[Uuid],
) -> bool {
	let identifier = id.to_string();
	match kind {
		PresaveAuthorizationKind::Sender => {
			scope_allows(snapshot.scope().sender_ids(), &[identifier])
				&& uuid_scope_allows(
					snapshot.scope().product_ids(),
					child_product_ids,
				) && uuid_scope_allows(snapshot.scope().study_ids(), child_study_ids)
		}
		PresaveAuthorizationKind::Product => {
			scope_allows(snapshot.scope().product_ids(), &[identifier])
				&& parent_scope_allows(
					snapshot.scope().sender_ids(),
					sender_presave_id,
				) && uuid_scope_allows(snapshot.scope().study_ids(), child_study_ids)
		}
		PresaveAuthorizationKind::Study => {
			scope_allows(snapshot.scope().study_ids(), &[identifier])
				&& parent_scope_allows(
					snapshot.scope().product_ids(),
					product_presave_id,
				) && parent_scope_allows(
				snapshot.scope().sender_ids(),
				sender_presave_id,
			)
		}
		PresaveAuthorizationKind::Receiver
		| PresaveAuthorizationKind::Reporter
		| PresaveAuthorizationKind::Narrative => true,
	}
}

fn uuid_scope_allows(assigned: &[String], available: &[Uuid]) -> bool {
	if assigned.is_empty() {
		return true;
	}
	available.iter().any(|candidate| {
		assigned
			.iter()
			.any(|assigned| assigned.eq_ignore_ascii_case(&candidate.to_string()))
	})
}

fn parent_scope_allows(assigned: &[String], parent_id: Option<Uuid>) -> bool {
	assigned.is_empty()
		|| parent_id.is_some_and(|parent_id| {
			scope_allows(assigned, &[parent_id.to_string()])
		})
}

fn case_lifecycle_allows(facts: &CaseFacts, kind: CaseMutationKind) -> bool {
	let status = facts.status.trim().to_ascii_lowercase();
	match kind {
		CaseMutationKind::Edit | CaseMutationKind::ChildEdit => status == "draft",
		CaseMutationKind::ReviewToggle => {
			matches!(status.as_str(), "draft" | "reviewed" | "validated")
		}
		CaseMutationKind::LockToggle => {
			matches!(status.as_str(), "draft" | "reviewed" | "validated")
				|| status == "locked"
		}
		CaseMutationKind::Delete => {
			!matches!(status.as_str(), "deleted" | "archived")
		}
		CaseMutationKind::Restore => status == "deleted",
		CaseMutationKind::Submission => {
			!matches!(status.as_str(), "locked" | "deleted" | "archived")
		}
		CaseMutationKind::WorkflowTransition => status != "locked",
	}
}

fn ensure_current_revisions(
	snapshot: &RequestAuthorizationSnapshot,
	current_organization_revision: i64,
	current_principal_revision: i64,
) -> Result<(), AuthorizationFactLoadError> {
	let version = snapshot.version();
	if version.organization_revision() != current_organization_revision
		|| version.principal_revision() != current_principal_revision
	{
		return Err(AuthorizationFactLoadError::SnapshotStale {
			snapshot_organization_revision: version.organization_revision(),
			current_organization_revision,
			snapshot_principal_revision: version.principal_revision(),
			current_principal_revision,
		});
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authorization::{
		BuiltInIdentityKind, GrantId, IdentityTraits, PolicySnapshotVersion,
		PrincipalScope, RequestAuthorizationSnapshot,
	};
	use std::collections::BTreeSet;
	use time::OffsetDateTime;
	use uuid::Uuid;

	fn snapshot(
		organization_revision: i64,
		principal_revision: i64,
		sender_ids: Vec<String>,
		product_ids: Vec<String>,
	) -> RequestAuthorizationSnapshot {
		let organization_id = Uuid::new_v4();
		RequestAuthorizationSnapshot::new(
			Uuid::new_v4(),
			organization_id,
			Uuid::new_v4(),
			IdentityTraits::new(Some(BuiltInIdentityKind::OperationalUser)),
			BTreeSet::<GrantId>::new(),
			PrincipalScope::new(sender_ids, product_ids, Vec::new(), false, None),
			PolicySnapshotVersion::new(
				"a".repeat(64),
				organization_id,
				organization_revision,
				principal_revision,
			),
			OffsetDateTime::now_utc(),
			None,
			"test".into(),
		)
	}

	#[test]
	fn draft_allows_export_receiver_preparation_without_changing_submission_lifecycle(
	) {
		let facts = CaseFacts {
			organization_id: Uuid::new_v4(),
			status: "draft".into(),
			sender_identifiers: Vec::new(),
			product_identifiers: Vec::new(),
			study_identifiers: Vec::new(),
		};

		assert!(case_lifecycle_allows(&facts, CaseMutationKind::Submission));
		let locked = CaseFacts {
			status: "locked".into(),
			..facts
		};
		assert!(!case_lifecycle_allows(
			&locked,
			CaseMutationKind::Submission
		));
	}

	#[test]
	fn mutation_facts_reject_a_stale_policy_snapshot() {
		let snapshot = snapshot(4, 7, Vec::new(), Vec::new());

		let error = ensure_current_revisions(&snapshot, 5, 7)
			.expect_err("changed organization revision must be stale");

		assert!(matches!(
			error,
			AuthorizationFactLoadError::SnapshotStale {
				snapshot_organization_revision: 4,
				current_organization_revision: 5,
				snapshot_principal_revision: 7,
				current_principal_revision: 7,
			}
		));
	}

	#[test]
	fn product_presave_requires_its_linked_sender_scope() {
		let allowed_sender_id = Uuid::new_v4();
		let blocked_sender_id = Uuid::new_v4();
		let product_id = Uuid::new_v4();
		let snapshot =
			snapshot(1, 1, vec![allowed_sender_id.to_string()], Vec::new());

		assert!(presave_within_scope(
			&snapshot,
			PresaveAuthorizationKind::Product,
			product_id,
			None,
			Some(allowed_sender_id),
			&[],
			&[],
		));
		assert!(!presave_within_scope(
			&snapshot,
			PresaveAuthorizationKind::Product,
			product_id,
			None,
			None,
			&[],
			&[],
		));
		assert!(!presave_within_scope(
			&snapshot,
			PresaveAuthorizationKind::Product,
			product_id,
			None,
			Some(blocked_sender_id),
			&[],
			&[],
		));
	}

	#[test]
	fn parent_scopes_are_required_for_presave_descendants() {
		let sender_id = Uuid::new_v4();
		let product_id = Uuid::new_v4();
		let study_id = Uuid::new_v4();
		let other_sender_id = Uuid::new_v4();
		let other_product_id = Uuid::new_v4();
		let sender_snapshot =
			snapshot(1, 1, vec![sender_id.to_string()], Vec::new());

		assert!(presave_within_scope(
			&sender_snapshot,
			PresaveAuthorizationKind::Study,
			study_id,
			Some(product_id),
			Some(sender_id),
			&[],
			&[],
		));
		assert!(!presave_within_scope(
			&sender_snapshot,
			PresaveAuthorizationKind::Study,
			study_id,
			Some(other_product_id),
			Some(other_sender_id),
			&[],
			&[],
		));

		let product_snapshot =
			snapshot(1, 1, Vec::new(), vec![product_id.to_string()]);
		assert!(presave_within_scope(
			&product_snapshot,
			PresaveAuthorizationKind::Sender,
			sender_id,
			None,
			None,
			&[product_id],
			&[study_id],
		));
		assert!(!presave_within_scope(
			&product_snapshot,
			PresaveAuthorizationKind::Sender,
			other_sender_id,
			None,
			None,
			&[other_product_id],
			&[],
		));
	}
}
