use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct As2GatewaySubmitResponse {
	remote_submission_id: Option<String>,
	ack: Option<EsgAckResponse>,
	latest_ack: Option<EsgAckResponse>,
	status: Option<String>,
}

fn require_ack1(
	ack: Option<EsgAckResponse>,
	gateway: &str,
	now: OffsetDateTime,
) -> Result<SubmissionAck> {
	let ack = ack.ok_or(Error::BadRequest {
		message: format!(
			"{gateway} submit response missing ACK1; remote submission remains pending acknowledgement"
		),
	})?;
	let level = ack.level.ok_or(Error::BadRequest {
		message: format!("{gateway} submit response ACK level is missing"),
	})?;
	if level != 1 {
		return Err(Error::BadRequest {
			message: format!(
				"{gateway} submit response must contain ACK1, received ACK{level}"
			),
		});
	}
	let success = ack.success.ok_or(Error::BadRequest {
		message: format!("{gateway} submit response ACK1 success is missing"),
	})?;
	if !success {
		return Err(Error::BadRequest {
			message: format!("{gateway} submit response contains unsuccessful ACK1"),
		});
	}
	Ok(SubmissionAck {
		level,
		success,
		code: ack.code,
		message: ack.message,
		received_at: now,
	})
}

pub(super) async fn submit_to_gateway(
	case_id: Uuid,
	xml: &str,
	authority: SubmissionAuthority,
) -> Result<GatewaySubmissionOutcome> {
	let now = OffsetDateTime::now_utc();
	if as2_submitter_url().is_some() {
		return Err(Error::BadRequest { message: "AS2 requires durable reservation; automatic gateway resend is disabled".into() });
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
	if !status.is_success() {
		return Err(Error::BadRequest {
			message: format!("FDA ESG submit failed ({status})"),
		});
	}
	let body_text = resp.text().await.map_err(|err| Error::BadRequest {
		message: format!("FDA ESG submit response read failed: {err}"),
	})?;

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
	let ack1 = require_ack1(parsed.ack, "FDA ESG", now)?;
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
		|| lower.contains("ack1")
		|| lower.contains("response is not valid json")
		|| lower.contains("rejected request (")
		|| lower.contains("submit failed ("))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn ack(level: Option<u8>, success: Option<bool>) -> EsgAckResponse {
		EsgAckResponse {
			level,
			success,
			code: Some("ACK1".to_string()),
			message: Some("accepted by gateway".to_string()),
		}
	}

	#[test]
	fn missing_ack_is_an_error_instead_of_synthetic_success() {
		let error = require_ack1(None, "FDA ESG", OffsetDateTime::now_utc())
			.expect_err("missing ACK must not be accepted");
		let message = error.to_string();
		assert!(message.contains("missing ACK1"), "{message}");
		assert!(!message.contains("ACK1_ACCEPTED"), "{message}");
	}

	#[test]
	fn ack_requires_explicit_level_and_success() {
		for response in [ack(None, Some(true)), ack(Some(1), None)] {
			assert!(
				require_ack1(Some(response), "AS2", OffsetDateTime::now_utc())
					.is_err()
			);
		}
	}

	#[test]
	fn explicit_successful_ack_is_preserved() {
		let result = require_ack1(
			Some(ack(Some(1), Some(true))),
			"AS2",
			OffsetDateTime::now_utc(),
		)
		.expect("explicit ACK1 should be accepted");
		assert_eq!(result.level, 1);
		assert!(result.success);
		assert_eq!(result.code.as_deref(), Some("ACK1"));
		assert_eq!(result.message.as_deref(), Some("accepted by gateway"));
	}

	#[test]
	fn unsuccessful_ack_is_not_reported_as_submission_success() {
		let error = require_ack1(
			Some(ack(Some(1), Some(false))),
			"FDA ESG",
			OffsetDateTime::now_utc(),
		)
		.expect_err("negative ACK must not enter successful outcome type");
		let message = error.to_string();
		assert!(message.contains("unsuccessful ACK1"));
		assert!(!message.contains("accepted by gateway"));
	}

	#[test]
	fn unconfigured_transport_does_not_select_a_gateway() {
		std::env::remove_var("AS2_SUBMITTER_URL");
		std::env::remove_var("FDA_ESG_ENABLED");
		std::env::remove_var("FDA_ESG_BASE_URL");
		let result = select_gateway_name(SubmissionAuthority::Fda);
		let error = result.expect_err("unconfigured transport must fail");
		assert!(error
			.to_string()
			.contains("no submission transport configured"));
	}

	#[test]
	fn missing_ack_is_not_retryable() {
		assert!(!is_retryable_submit_error(
			"FDA ESG submit response missing ACK1; remote submission remains pending acknowledgement"
		));
	}
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

// Dispatch exactly once; subsequent reconciliation uses the read-only status endpoint.
pub(super) async fn request_as2_state(
	case_id: Uuid,
	authority: SubmissionAuthority,
	xml: Option<&str>,
) -> Result<As2GatewaySubmitResponse> {
	let base = as2_submitter_url().ok_or(Error::BadRequest {
		message: "AS2_SUBMITTER_URL is required".into(),
	})?;
	let token = as2_submitter_token()?;
	let path = if xml.is_some() {
		"submit"
	} else {
		"submissions/status"
	};
	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(parse_timeout_secs(
			"AS2_SUBMITTER_TIMEOUT_SECS",
			30,
		)))
		.build()
		.map_err(|e| Error::BadRequest {
			message: e.to_string(),
		})?;
	let response = client
		.post(format!("{}/{path}", base.trim_end_matches('/')))
		.header("x-api-token", token.trim())
		.json(
			&json!({"caseId": case_id.to_string(), "authority": authority.as_str(),
            "idempotencyKey": case_id.to_string(), "xmlPayload": xml,
            "callbackUrl": std::env::var("AS2_ACK_CALLBACK_URL").ok()}),
		)
		.send()
		.await
		.map_err(|e| Error::BadRequest {
			message: format!("AS2 outcome unavailable: {e}"),
		})?;
	let http_status = response.status();
	let parsed = response
		.json::<As2GatewaySubmitResponse>()
		.await
		.map_err(|e| Error::BadRequest {
			message: format!("AS2 state unavailable ({http_status}): {e}"),
		})?;
	if !http_status.is_success() && !matches!(http_status.as_u16(), 409 | 502) {
		return Err(Error::BadRequest {
			message: format!("AS2 state unavailable ({http_status})"),
		});
	}
	parsed.observed()?;
	Ok(parsed)
}

impl As2GatewaySubmitResponse {
	pub(super) fn observed(
		&self,
	) -> Result<(String, SubmissionStatus, Option<SubmissionAck>)> {
		let remote = self
			.remote_submission_id
			.as_ref()
			.filter(|v| !v.trim().is_empty())
			.ok_or(Error::BadRequest {
				message: "AS2 state missing remote message ID".into(),
			})?
			.clone();
		let ack = self.latest_ack.as_ref().or(self.ack.as_ref());
		if let Some(ack) = ack {
			let level = ack.level.ok_or(Error::BadRequest {
				message: "AS2 ACK level missing".into(),
			})?;
			let success = ack.success.ok_or(Error::BadRequest {
				message: "AS2 ACK outcome missing".into(),
			})?;
			let status = status_from_ack(level, success)?;
			return Ok((
				remote,
				status,
				Some(SubmissionAck {
					level,
					success,
					code: ack.code.clone(),
					message: ack.message.clone(),
					received_at: OffsetDateTime::now_utc(),
				}),
			));
		}
		let status = match self.status.as_deref() {
			Some("dispatch_unknown") => SubmissionStatus::DispatchUnknown,
			Some("submitted_ack1_pending") => SubmissionStatus::SubmittedAck1Pending,
			_ => return Err(Error::BadRequest {
				message:
					"AS2 state has no verifiable acknowledgement or pending status"
						.into(),
			}),
		};
		Ok((remote, status, None))
	}
}

pub(super) fn as2_submitter_token() -> Result<String> {
	let token =
		std::env::var("AS2_SUBMITTER_TOKEN").map_err(|_| Error::BadRequest {
			message: "AS2_SUBMITTER_TOKEN is required".into(),
		})?;
	if token.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "AS2_SUBMITTER_TOKEN is empty".into(),
		});
	}
	Ok(token)
}
