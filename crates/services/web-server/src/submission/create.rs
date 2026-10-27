use super::*;

// Called inside the authorized mutation; the caller must commit before dispatch.
pub async fn create_submission_idempotent(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	idempotency_key: Option<String>,
) -> Result<(SubmissionRecord, bool)> {
	let key = idempotency_key.map(|v| v.trim().to_string());
	if key.as_ref().is_some_and(|v| v.is_empty()) {
		return Err(Error::BadRequest {
			message: "idempotency key must not be empty".into(),
		});
	}
	if let Some(key) = key.as_deref() {
		if let Some(id) =
			find_submission_idempotency(ctx, mm, case_id, authority, key).await?
		{
			let record =
				get_submission(ctx, mm, id)
					.await?
					.ok_or(Error::BadRequest {
						message: "idempotent submission reference not found".into(),
					})?;
			return Ok((record, false));
		}
	}
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	if case.status.eq_ignore_ascii_case("submitted") {
		return Err(Error::BadRequest {
			message: "case has already been submitted".into(),
		});
	}
	let export_authority = match authority {
		SubmissionAuthority::Fda => RegulatoryAuthority::Fda,
		SubmissionAuthority::Mfds => RegulatoryAuthority::Mfds,
	};
	let xml = prepare_submission_xml(ctx, mm, case_id, export_authority).await?;
	let gateway = select_gateway_name(authority)?;
	if gateway.starts_with("as2-") {
		as2_submitter_token()?;
	}

	let id = Uuid::new_v4();
	let now = OffsetDateTime::now_utc();
	mm.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	let result = async {
        set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role()).await?;
        set_compliance_context_dbx(mm.dbx(), ctx.change_reason(), ctx.change_category(), ctx.e_signature_id()).await?;
        let changed = mm.dbx().execute(sqlx::query(
            "UPDATE cases SET status='submitted',submitted_by=$2,submitted_at=$3,raw_xml=$4,
             dirty_c=false,dirty_d=false,dirty_e=false,dirty_f=false,dirty_g=false,dirty_h=false,updated_at=now()
             WHERE id=$1 AND status <> 'submitted'")
            .bind(case_id).bind(ctx.user_id()).bind(now).bind(xml.as_bytes())).await.map_err(lib_core::model::Error::from)?;
        if changed != 1 { return Err(Error::BadRequest { message: "case has already been submitted".into() }); }
        mm.dbx().execute(sqlx::query("INSERT INTO case_submissions
            (id,case_id,gateway,authority,submitted_xml,remote_submission_id,status,xml_bytes,submitted_by,submitted_at)
            VALUES ($1,$2,$3,$4,$5,NULL,'dispatch_unknown',$6,$7,$8)")
            .bind(id).bind(case_id).bind(&gateway).bind(authority.as_str()).bind(xml.as_bytes())
            .bind(i32::try_from(xml.len()).map_err(|_| Error::BadRequest { message: "submission XML is too large".into() })?)
            .bind(ctx.user_id()).bind(now)).await.map_err(lib_core::model::Error::from)?;
        if let Some(key) = key.as_deref() {
            insert_submission_idempotency(ctx, mm, case_id, authority, key, id, ctx.user_id()).await?;
        }
        append_submission_event(mm, id, "submission_created", Some(json!({"case_id":case_id,"gateway":gateway,"authority":authority.as_str(),"status":"dispatch_unknown"}))).await?;
        let next_lookup = if gateway.starts_with("as2-") { Some(now + time::Duration::minutes(1)) } else { None };
        upsert_dispatch_state_submit_failure(mm,id,now,0,"dispatch outcome not yet observed; do not resend",next_lookup).await?;
        Ok::<_, Error>(())
    }.await;
	match result {
		Ok(()) => mm
			.dbx()
			.commit_txn()
			.await
			.map_err(lib_core::model::Error::from)?,
		Err(error) => {
			mm.dbx()
				.rollback_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			return Err(error);
		}
	}
	let record = get_submission(ctx, mm, id)
		.await?
		.ok_or(Error::BadRequest {
			message: "reserved submission missing".into(),
		})?;
	Ok((record, true))
}

// Only the request that created the reservation calls this, after authorization commits.
pub async fn dispatch_reserved_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	record: SubmissionRecord,
) -> Result<SubmissionRecord> {
	let (authority, xml) = get_submission_artifact(ctx, mm, record.id).await?;
	let xml = std::str::from_utf8(&xml).map_err(|_| Error::BadRequest {
		message: "stored submission XML is not UTF-8".into(),
	})?;
	let outcome = match record.gateway.as_str() {
		"as2-submitter-http-fda" | "as2-submitter-http-mfds" => {
			request_as2_state(record.case_id, authority, Some(xml)).await
		}
		"fda-esg-nextgen-api" if authority == SubmissionAuthority::Fda => {
			gateway::submit_to_gateway(xml, authority)
				.await
				.map(|outcome| As2GatewaySubmitResponse::from(outcome))
		}
		_ => Err(Error::BadRequest {
			message: "unsupported stored submission transport".into(),
		}),
	};
	persist_submission_observation(ctx, mm, record.id, outcome).await?;
	get_submission(ctx, mm, record.id)
		.await?
		.ok_or(Error::BadRequest {
			message: "reserved submission missing".into(),
		})
}

pub async fn get_submission_artifact(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<(SubmissionAuthority, Vec<u8>)> {
	let read_mm = mm.new_with_txn()?;
	read_mm
		.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	let result = async {
        set_full_context_dbx(read_mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role()).await?;
        let (authority, xml) = read_mm.dbx().fetch_one(sqlx::query_as::<_, (Option<String>, Option<Vec<u8>>)>(
            "SELECT authority, submitted_xml FROM case_submissions WHERE id=$1").bind(id)).await.map_err(lib_core::model::Error::from)?;
        let authority = authority.as_deref().and_then(SubmissionAuthority::parse).ok_or(Error::BadRequest { message: "submission authority requires reconciliation".into() })?;
        let xml = xml.filter(|v| !v.is_empty()).ok_or(Error::BadRequest { message: "original submission XML unavailable; re-export is not a submission artifact".into() })?;
        Ok::<_, Error>((authority, xml))
    }.await;
	match result {
		Ok(value) => {
			read_mm
				.dbx()
				.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Ok(value)
		}
		Err(error) => {
			read_mm
				.dbx()
				.rollback_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Err(error)
		}
	}
}

pub async fn list_by_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<SubmissionRecord>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if let Err(err) = set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let rows = list_submission_rows_by_case(mm, case_id).await?;
	let mut out = Vec::with_capacity(rows.len());
	for row in rows {
		let acks = list_ack_rows(mm, row.id).await?;
		out.push(compose_submission_record(row, acks)?);
	}
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(out)
}

pub async fn get_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<Option<SubmissionRecord>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if let Err(err) = set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let Some(row) = get_submission_row(mm, id).await? else {
		mm.dbx()
			.commit_txn()
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		return Ok(None);
	};
	let acks = list_ack_rows(mm, id).await?;
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(Some(compose_submission_record(row, acks)?))
}

pub(super) async fn persist_submission_observation(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
	outcome: Result<As2GatewaySubmitResponse>,
) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	mm.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	let result = async {
        set_full_context_dbx(mm.dbx(),ctx.user_id(),ctx.organization_id(),ctx.role()).await.map_err(lib_core::model::Error::from)?;
        set_compliance_context_dbx(mm.dbx(),ctx.change_reason(),ctx.change_category(),ctx.e_signature_id()).await.map_err(lib_core::model::Error::from)?;
        let row = mm.dbx().fetch_one(sqlx::query_as::<_,CaseSubmissionRow>(
            "SELECT id,case_id,gateway,authority,remote_submission_id,status,xml_bytes,submitted_by,submitted_at
            FROM case_submissions WHERE id=$1 FOR UPDATE").bind(id)).await.map_err(lib_core::model::Error::from)?;
        let current = status_from_db(&row.status)?;
        match outcome {
            Ok(observed) => {
                let (remote,incoming,ack) = observed.observed()?;
                if let Some(existing) = &row.remote_submission_id {
                    if existing != &remote { return Err(Error::BadRequest { message: "AS2 remote identity changed".into() }); }
                }
                let merged = if ack.as_ref().is_some_and(|a| a.level < submission_status_rank(&current)) {
                    current.clone()
                } else { merge_submission_status(&current,&incoming) };
                mm.dbx().execute(sqlx::query("UPDATE case_submissions SET remote_submission_id=$2,status=$3,updated_at=now() WHERE id=$1")
                    .bind(id).bind(&remote).bind(status_to_db(&merged))).await.map_err(lib_core::model::Error::from)?;
                if let Some(a) = ack {
                    if !ack_event_exists(mm,id,a.level as i16,a.success,a.code.as_deref(),a.message.as_deref()).await? {
                        mm.dbx().execute(sqlx::query("INSERT INTO submission_acks(submission_id,ack_level,success,ack_code,ack_message,received_at,raw_payload) VALUES($1,$2,$3,$4,$5,$6,$7)")
                            .bind(id).bind(a.level as i16).bind(a.success).bind(&a.code).bind(&a.message).bind(now)
                            .bind(json!({"source":"gateway_observation","gateway":row.gateway,"level":a.level,"success":a.success}))).await.map_err(lib_core::model::Error::from)?;
                        append_submission_event(mm,id,"ack_recorded",Some(json!({"source":"gateway_observation","gateway":row.gateway,"ack_level":a.level,"success":a.success,"ack_code":a.code,"ack_message":a.message}))).await?;
                    }
                }
                if merged != current { append_submission_event(mm,id,"status_changed",Some(json!({"from":status_to_db(&current),"to":status_to_db(&merged)}))).await?; }
                if is_submission_terminal(&merged) || (merged == SubmissionStatus::Ack3Received && row.authority.as_deref() == Some("fda")) {
                    mark_dispatch_terminal(mm,id,now).await?;
                } else {
                    upsert_dispatch_state_submit_failure(mm,id,now,1,"awaiting acknowledgement; automatic resend disabled",if row.gateway.starts_with("as2-") { Some(now+time::Duration::minutes(1)) } else { None }).await?;
                }
            }
            Err(error) => {
                append_submission_event(mm,id,"submission_outcome_unavailable",Some(json!({"error":error.to_string(),"action":if row.gateway.starts_with("as2-") { "status_lookup_only" } else { "manual_reconciliation" }}))).await?;
                if !is_submission_terminal(&current) && !(current == SubmissionStatus::Ack3Received && row.authority.as_deref() == Some("fda")) {
                    upsert_dispatch_state_submit_failure(mm,id,now,1,&error.to_string(),if row.gateway.starts_with("as2-") { Some(now+time::Duration::minutes(1)) } else { None }).await?;
                }
            }
        }
        Ok::<_,Error>(())
    }.await;
	match result {
		Ok(()) => mm
			.dbx()
			.commit_txn()
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e))),
		Err(e) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(e)
		}
	}
}
