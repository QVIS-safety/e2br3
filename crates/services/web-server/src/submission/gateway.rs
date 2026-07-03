use super::*;

pub(super) async fn submit_to_gateway(
	case_id: Uuid,
	xml: &str,
	authority: SubmissionAuthority,
) -> Result<GatewaySubmissionOutcome> {
	let now = OffsetDateTime::now_utc();
	if let Some(base_url) = as2_submitter_url() {
		let submit_url = format!("{}/submit", base_url.trim_end_matches('/'));
		let timeout_secs = parse_timeout_secs("AS2_SUBMITTER_TIMEOUT_SECS", 30);
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(timeout_secs))
			.build()
			.map_err(|err| Error::BadRequest {
				message: format!("failed to initialize AS2 submitter client: {err}"),
			})?;
		let callback_url = std::env::var("AS2_ACK_CALLBACK_URL").ok();
		let mut req = client.post(&submit_url);
		if let Ok(token) = std::env::var("AS2_SUBMITTER_TOKEN")
			.or_else(|_| std::env::var("AS2_CALLBACK_TOKEN"))
		{
			let token = token.trim();
			if !token.is_empty() {
				req = req
					.header("x-api-token", token)
					.header("x-callback-token", token)
					.header(AUTHORIZATION, format!("Bearer {token}"));
			}
		}
		let resp = req
			.json(&json!({
				"caseId": case_id.to_string(),
				"authority": authority.as_str(),
				"xmlPayload": xml,
				"callbackUrl": callback_url,
			}))
			.send()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("AS2 submitter request failed: {err}"),
			})?;
		let status = resp.status();
		let body_text = resp.text().await.map_err(|err| Error::BadRequest {
			message: format!("AS2 submitter response read failed: {err}"),
		})?;
		if !status.is_success() {
			let body_snippet = body_text.chars().take(200).collect::<String>();
			return Err(Error::BadRequest {
				message: format!(
					"AS2 submitter rejected request ({status}): {body_snippet}"
				),
			});
		}
		let parsed: As2SubmitResponse =
			serde_json::from_str(&body_text).map_err(|err| Error::BadRequest {
				message: format!("AS2 submitter response is not valid JSON: {err}"),
			})?;
		let remote_submission_id = parsed
			.remote_submission_id
			.or(parsed.submission_id)
			.ok_or(Error::BadRequest {
				message:
					"AS2 submitter response missing remote submission identifier"
						.to_string(),
			})?;
		let ack_message = match (parsed.status, parsed.authority) {
			(Some(status), Some(authority)) => {
				Some(format!("AS2 accepted: {status} ({authority})"))
			}
			(Some(status), None) => Some(format!("AS2 accepted: {status}")),
			(None, Some(authority)) => Some(format!("AS2 accepted ({authority})")),
			(None, None) => None,
		};
		return Ok(GatewaySubmissionOutcome {
			gateway: "as2-submitter-http".to_string(),
			remote_submission_id,
			ack1: SubmissionAck {
				level: 1,
				success: true,
				code: Some("ACK1_ACCEPTED".to_string()),
				message: ack_message,
				received_at: now,
			},
		});
	}

	if allow_mock_submission() {
		let submission_id = Uuid::new_v4();
		return Ok(GatewaySubmissionOutcome {
			gateway: "fda-esg-nextgen-mock".to_string(),
			remote_submission_id: format!(
				"{}-MOCK-{}",
				authority.as_str().to_ascii_uppercase(),
				submission_id.simple().to_string().to_uppercase()
			),
			ack1: SubmissionAck {
				level: 1,
				success: true,
				code: Some("ACK1_ACCEPTED".to_string()),
				message: Some("Upload accepted by mock FDA gateway".to_string()),
				received_at: now,
			},
		});
	}
	if !is_esg_enabled() {
		return Err(Error::BadRequest {
			message: "no submission transport configured: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		});
	}
	if authority != SubmissionAuthority::Fda {
		return Err(Error::BadRequest {
			message:
				"FDA ESG transport only supports authority=fda; configure AS2 for MFDS submissions"
					.to_string(),
		});
	}

	let base_url =
		std::env::var("FDA_ESG_BASE_URL").map_err(|_| Error::BadRequest {
			message: "FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string(),
		})?;
	let submit_path = std::env::var("FDA_ESG_SUBMIT_PATH")
		.unwrap_or_else(|_| "/submissions".to_string());
	let submit_url = format!(
		"{}/{}",
		base_url.trim_end_matches('/'),
		submit_path.trim_start_matches('/')
	);
	let timeout_secs = parse_timeout_secs("FDA_ESG_TIMEOUT_SECS", 30);
	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(timeout_secs))
		.build()
		.map_err(|err| Error::BadRequest {
			message: format!("failed to initialize FDA ESG client: {err}"),
		})?;

	let mut headers = HeaderMap::new();
	if let Ok(token) = std::env::var("FDA_ESG_BEARER_TOKEN") {
		let value = format!("Bearer {}", token.trim());
		let hv = HeaderValue::from_str(&value).map_err(|_| Error::BadRequest {
			message: "invalid FDA_ESG_BEARER_TOKEN".to_string(),
		})?;
		headers.insert(AUTHORIZATION, hv);
	}
	if let Ok(api_key) = std::env::var("FDA_ESG_API_KEY") {
		let hv = HeaderValue::from_str(api_key.trim()).map_err(|_| {
			Error::BadRequest {
				message: "invalid FDA_ESG_API_KEY".to_string(),
			}
		})?;
		headers.insert("x-api-key", hv);
	}

	let resp = client
		.post(&submit_url)
		.headers(headers)
		.json(&json!({ "xml": xml }))
		.send()
		.await
		.map_err(|err| Error::BadRequest {
			message: format!("FDA ESG submit request failed: {err}"),
		})?;
	let status = resp.status();
	let body_text = resp.text().await.map_err(|err| Error::BadRequest {
		message: format!("FDA ESG submit response read failed: {err}"),
	})?;
	if !status.is_success() {
		let body_snippet = body_text.chars().take(200).collect::<String>();
		return Err(Error::BadRequest {
			message: format!("FDA ESG submit failed ({status}): {body_snippet}"),
		});
	}

	let parsed: EsgSubmitResponse =
		serde_json::from_str(&body_text).map_err(|err| Error::BadRequest {
			message: format!("FDA ESG submit response is not valid JSON: {err}"),
		})?;
	let remote_submission_id = parsed
		.remote_submission_id
		.or(parsed.submission_id)
		.or(parsed.id)
		.ok_or(Error::BadRequest {
			message: "FDA ESG submit response missing remote submission identifier"
				.to_string(),
		})?;
	let ack = parsed.ack.unwrap_or(EsgAckResponse {
		level: Some(1),
		success: Some(true),
		code: Some("ACK1_ACCEPTED".to_string()),
		message: Some(
			"Submitted to FDA ESG; awaiting downstream ACK updates".to_string(),
		),
		received_at: None,
	});
	let ack1 = SubmissionAck {
		level: ack.level.unwrap_or(1),
		success: ack.success.unwrap_or(true),
		code: ack.code,
		message: ack.message,
		received_at: now,
	};
	Ok(GatewaySubmissionOutcome {
		gateway: "fda-esg-nextgen-api".to_string(),
		remote_submission_id,
		ack1,
	})
}

pub(super) fn select_gateway_name(authority: SubmissionAuthority) -> Result<String> {
	if as2_submitter_url().is_some() {
		return Ok("as2-submitter-http".to_string());
	}
	if allow_mock_submission() {
		return Ok("fda-esg-nextgen-mock".to_string());
	}
	if !is_esg_enabled() {
		return Err(Error::BadRequest {
			message: "no submission transport configured: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		});
	}
	if authority != SubmissionAuthority::Fda {
		return Err(Error::BadRequest {
			message:
				"FDA ESG transport only supports authority=fda; configure AS2 for MFDS submissions"
					.to_string(),
		});
	}
	let _ = std::env::var("FDA_ESG_BASE_URL").map_err(|_| Error::BadRequest {
		message: "FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string(),
	})?;
	Ok("fda-esg-nextgen-api".to_string())
}

pub(super) fn submission_max_attempts() -> u32 {
	std::env::var("SUBMISSION_MAX_ATTEMPTS")
		.ok()
		.and_then(|v| v.trim().parse::<u32>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(1)
}

pub(super) fn submission_retry_base_ms() -> u64 {
	std::env::var("SUBMISSION_RETRY_BASE_MS")
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(500)
}

pub(super) fn submission_retry_max_ms() -> u64 {
	std::env::var("SUBMISSION_RETRY_MAX_MS")
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(10_000)
}

pub(super) fn backoff_ms_for_attempt(attempt_number: u32) -> u64 {
	let base = submission_retry_base_ms();
	let max = submission_retry_max_ms();
	let shift = attempt_number.saturating_sub(1).min(16);
	let pow = 1u64 << shift;
	base.saturating_mul(pow).min(max)
}

pub(super) fn is_retryable_submit_error(msg: &str) -> bool {
	let lower = msg.to_ascii_lowercase();
	!(lower.contains("missing remote submission identifier")
		|| lower.contains("response is not valid json")
		|| lower.contains("rejected request (")
		|| lower.contains("submit failed ("))
}

pub(super) struct GatewayDispatchFailure {
	pub(super) message: String,
	pub(super) attempts: u32,
	pub(super) next_retry_at: Option<OffsetDateTime>,
}

pub(super) async fn submit_to_gateway_with_retry(
	case_id: Uuid,
	xml: &str,
	authority: SubmissionAuthority,
) -> core::result::Result<(GatewaySubmissionOutcome, u32), GatewayDispatchFailure> {
	let max_attempts = submission_max_attempts();
	let mut last_error = "submission failed".to_string();

	for attempt in 1..=max_attempts {
		match submit_to_gateway(case_id, xml, authority).await {
			Ok(outcome) => return Ok((outcome, attempt)),
			Err(err) => {
				last_error = err.to_string();
				let retryable = is_retryable_submit_error(&last_error);
				if attempt >= max_attempts || !retryable {
					let next_retry_at = if retryable {
						Some(
							OffsetDateTime::now_utc()
								+ time::Duration::milliseconds(
									backoff_ms_for_attempt(attempt) as i64,
								),
						)
					} else {
						None
					};
					return Err(GatewayDispatchFailure {
						message: last_error,
						attempts: attempt,
						next_retry_at,
					});
				}
				sleep(Duration::from_millis(backoff_ms_for_attempt(attempt))).await;
			}
		}
	}

	Err(GatewayDispatchFailure {
		message: last_error,
		attempts: max_attempts,
		next_retry_at: None,
	})
}
