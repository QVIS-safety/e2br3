use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{ModelManager, Result};
use modql::field::Fields;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SubmissionReceiverOption {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub authority: String,
	pub sequence_number: i32,
	pub receiver_label: String,
	pub condition_page: String,
	pub condition_field_code: String,
	pub condition_operator: String,
	pub condition_value_code: String,
	pub condition_value_label: Option<String>,
	pub batch_receiver_identifier: String,
	pub message_receiver_identifier: String,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Option<Uuid>,
	pub updated_by: Option<Uuid>,
}

pub struct SubmissionReceiverOptionBmc;

impl DbBmc for SubmissionReceiverOptionBmc {
	const TABLE: &'static str = "submission_receiver_options";
}

impl SubmissionReceiverOptionBmc {
	pub async fn list_by_authority(
		ctx: &Ctx,
		mm: &ModelManager,
		authority: &str,
	) -> Result<Vec<SubmissionReceiverOption>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}

		if let Err(err) = Self::ensure_defaults_for_org(ctx, mm).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}

		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, SubmissionReceiverOption>(
					r#"
					SELECT *
					FROM submission_receiver_options
					WHERE organization_id = $1
					  AND authority = $2
					  AND deleted = false
					ORDER BY sequence_number ASC, receiver_label ASC
					"#,
				)
				.bind(ctx.organization_id())
				.bind(authority),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;
		Ok(rows)
	}

	async fn ensure_defaults_for_org(ctx: &Ctx, mm: &ModelManager) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO submission_receiver_options (
						organization_id,
						authority,
						sequence_number,
						receiver_label,
						condition_field_code,
						condition_value_code,
						condition_value_label,
						batch_receiver_identifier,
						message_receiver_identifier,
						created_by,
						updated_by
					)
					SELECT $1, v.authority, v.sequence_number, v.receiver_label,
						   v.condition_field_code, v.condition_value_code,
						   v.condition_value_label, v.batch_receiver_identifier,
						   v.message_receiver_identifier, $2, $2
					FROM (VALUES
						('fda', 1, 'FDA(CDER IND)', 'FDA_REPORT_TYPE', '1', 'CDER IND', 'ZZFDA_PREMKT', 'CDER_IND'),
						('fda', 2, 'FDA(CDER IND-exempt BA/BE)', 'FDA_REPORT_TYPE', '2', 'CDER IND-exempt BA/BE', 'ZZFDA_PREMKT', 'CDER_IND_EXEMPT_BA_BE'),
						('fda', 3, 'FDA(CBER IND)', 'FDA_REPORT_TYPE', '3', 'CBER IND', 'ZZFDA_PREMKT', 'CBER_IND'),
						('fda', 4, 'FDA(Postmarket)', 'FDA_REPORT_TYPE', '4', 'Postmarket', 'ZZFDA', 'CDER'),
						('mfds', 1, 'MFDS(CT)', 'MFDS_REPORT_TYPE', '1', '임상시험계획의 승인을 받은 자', 'MFDS_CT', 'CT'),
						('mfds', 2, 'MFDS(CU)', 'MFDS_REPORT_TYPE', '2', '임상시험용의약품의 치료목적 사용승인을 받은 자', 'MFDS_CU', 'CU'),
						('mfds', 3, 'MFDS(KR)', 'MFDS_REPORT_TYPE', '3', '시판 후 이상사례 국내보고', 'MFDS', 'KR'),
						('mfds', 4, 'MFDS(FR)', 'MFDS_REPORT_TYPE', '4', '시판 후 이상사례 국외보고', 'MFDS_FR', 'FR'),
						('mfds', 5, 'MFDS(CF)', 'MFDS_REPORT_TYPE', '5', '임상시험계획의 승인을 받은 자 (국외)', 'MFDS_CF', 'CF')
					) AS v(authority, sequence_number, receiver_label, condition_field_code, condition_value_code, condition_value_label, batch_receiver_identifier, message_receiver_identifier)
					ON CONFLICT DO NOTHING
					"#,
				)
				.bind(ctx.organization_id())
				.bind(ctx.user_id()),
			)
			.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					UPDATE submission_receiver_options
					SET condition_value_label = CASE receiver_label
							WHEN 'MFDS(CT)' THEN '임상시험계획의 승인을 받은 자'
							WHEN 'MFDS(CU)' THEN '임상시험용의약품의 치료목적 사용승인을 받은 자'
							WHEN 'MFDS(KR)' THEN '시판 후 이상사례 국내보고'
							WHEN 'MFDS(FR)' THEN '시판 후 이상사례 국외보고'
							ELSE condition_value_label
						END,
						updated_by = $2
					WHERE organization_id = $1
					  AND authority = 'mfds'
					  AND condition_field_code = 'MFDS_REPORT_TYPE'
					  AND (
						(receiver_label = 'MFDS(CT)' AND condition_value_code = '1' AND condition_value_label = 'CT')
						OR (receiver_label = 'MFDS(CU)' AND condition_value_code = '2' AND condition_value_label = 'CU')
						OR (receiver_label = 'MFDS(KR)' AND condition_value_code = '3' AND condition_value_label = 'KR')
						OR (receiver_label = 'MFDS(FR)' AND condition_value_code = '4' AND condition_value_label = 'FR')
					  )
					"#,
				)
				.bind(ctx.organization_id())
				.bind(ctx.user_id()),
			)
			.await?;
		Ok(())
	}
}
