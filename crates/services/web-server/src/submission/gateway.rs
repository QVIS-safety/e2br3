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

pub(super) fn select_gateway_name(authority: SubmissionAuthority) -> Result<String> {
	Ok(format!("as2-submitter-http-{}", authority.as_str()))
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
}

pub(super) struct PreparedGatewayRequest {
	client: reqwest::Client,
	request: reqwest::Request,
	as2: bool,
}

pub(super) struct DispatchFailure {
	pub error: Error,
	pub not_sent: bool,
}

pub(super) fn prepare_submission_request(
	case_id: Uuid,
	authority: SubmissionAuthority,
	xml: &str,
	gateway: &str,
	profile_id: Uuid,
	profile_version: i64,
	routing_id: &str,
) -> Result<PreparedGatewayRequest> {
	match gateway {
		"as2-submitter-http-fda" | "as2-submitter-http-mfds" => prepare_as2_request(
			case_id,
			authority,
			Some(xml),
			Some(profile_id),
			Some(profile_version),
			Some(routing_id),
		),
		_ => Err(Error::BadRequest {
			message: "unsupported submission transport".into(),
		}),
	}
}

fn prepare_as2_request(
	case_id: Uuid,
	authority: SubmissionAuthority,
	xml: Option<&str>,
	profile_id: Option<Uuid>,
	profile_version: Option<i64>,
	routing_id: Option<&str>,
) -> Result<PreparedGatewayRequest> {
	let base = as2_submitter_url()?.ok_or(Error::Configuration {
		key: "AS2_SUBMITTER_URL",
		message: "submission transport is not configured".into(),
	})?;
	let token = as2_submitter_token()?;
	let callback = std::env::var("AS2_ACK_CALLBACK_URL").map_err(|error| {
		Error::Configuration {
			key: "AS2_ACK_CALLBACK_URL",
			message: error.to_string(),
		}
	})?;
	let callback_url =
		reqwest::Url::parse(&callback).map_err(|_| Error::Configuration {
			key: "AS2_ACK_CALLBACK_URL",
			message: "callback must be an absolute HTTP(S) URL".into(),
		})?;
	if !matches!(callback_url.scheme(), "http" | "https")
		|| callback_url.host_str().is_none()
	{
		return Err(Error::Configuration {
			key: "AS2_ACK_CALLBACK_URL",
			message: "callback must be an absolute HTTP(S) URL".into(),
		});
	}
	let path = if xml.is_some() {
		"submit"
	} else {
		"submissions/status"
	};
	let client = reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.timeout(Duration::from_secs(parse_timeout_secs(
			"AS2_SUBMITTER_TIMEOUT_SECS",
		)?))
		.build()
		.map_err(|error| Error::BadRequest {
			message: error.to_string(),
		})?;
	let request = client.post(format!("{}/{path}", base.trim_end_matches('/')))
        .header("x-api-token", token.trim())
		.json(&json!({"caseId":case_id.to_string(),"authority":authority.as_str(),"idempotencyKey":case_id.to_string(),"xmlPayload":xml,"callbackUrl":callback,"profileId":profile_id,"profileVersion":profile_version,"routingId":routing_id}))
        .build().map_err(|error| Error::BadRequest { message: format!("invalid AS2 request: {error}") })?;
	if !matches!(request.url().scheme(), "http" | "https")
		|| request.url().host_str().is_none()
	{
		return Err(Error::Configuration {
			key: "AS2_SUBMITTER_URL",
			message: "submitter must be an absolute HTTP(S) URL".into(),
		});
	}
	Ok(PreparedGatewayRequest {
		client,
		request,
		as2: true,
	})
}

impl PreparedGatewayRequest {
	pub(super) async fn execute(
		self,
	) -> std::result::Result<As2GatewaySubmitResponse, DispatchFailure> {
		let response = self.client.execute(self.request).await.map_err(|error| {
			DispatchFailure {
				not_sent: error.is_connect() || error.is_builder(),
				error: Error::BadRequest {
					message: format!("Submission transport failed: {error}"),
				},
			}
		})?;
		parse_gateway_response(response, self.as2)
			.await
			.map_err(|error| DispatchFailure {
				error,
				not_sent: false,
			})
	}
}

async fn parse_gateway_response(
	response: reqwest::Response,
	as2: bool,
) -> Result<As2GatewaySubmitResponse> {
	let http_status = response.status();
	if as2 {
		let parsed =
			response
				.json::<As2GatewaySubmitResponse>()
				.await
				.map_err(|error| Error::BadRequest {
					message: format!(
						"AS2 state unavailable ({http_status}): {error}"
					),
				})?;
		if !http_status.is_success() && !matches!(http_status.as_u16(), 409 | 502) {
			return Err(Error::BadRequest {
				message: format!("AS2 state unavailable ({http_status})"),
			});
		}
		parsed.observed()?;
		return Ok(parsed);
	}
	if !http_status.is_success() {
		return Err(Error::BadRequest {
			message: format!("FDA ESG submit failed ({http_status})"),
		});
	}
	let parsed = response
		.json::<EsgSubmitResponse>()
		.await
		.map_err(|error| Error::BadRequest {
			message: format!("FDA ESG submit response is not valid JSON: {error}"),
		})?;
	let remote_submission_id = parsed
		.remote_submission_id
		.or(parsed.submission_id)
		.or(parsed.id)
		.ok_or(Error::BadRequest {
			message: "FDA ESG submit response missing remote submission identifier"
				.into(),
		})?;
	let ack1 = require_ack1(parsed.ack, "FDA ESG", OffsetDateTime::now_utc())?;
	Ok(GatewaySubmissionOutcome {
		remote_submission_id,
		ack1,
	}
	.into())
}

// Reconciliation observes status only; a lookup connection failure says nothing about the original send.
pub(super) async fn request_as2_state(
	case_id: Uuid,
	authority: SubmissionAuthority,
	xml: Option<&str>,
) -> Result<As2GatewaySubmitResponse> {
	prepare_as2_request(case_id, authority, xml, None, None, None)?
		.execute()
		.await
		.map_err(|failure| failure.error)
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
		std::env::var("AS2_SUBMITTER_TOKEN").map_err(|_| Error::Configuration {
			key: "AS2_SUBMITTER_TOKEN",
			message: "AS2_SUBMITTER_TOKEN is required".into(),
		})?;
	if token.trim().is_empty() {
		return Err(Error::Configuration {
			key: "AS2_SUBMITTER_TOKEN",
			message: "AS2_SUBMITTER_TOKEN is empty".into(),
		});
	}
	Ok(token)
}

impl From<GatewaySubmissionOutcome> for As2GatewaySubmitResponse {
	fn from(value: GatewaySubmissionOutcome) -> Self {
		Self {
			remote_submission_id: Some(value.remote_submission_id),
			ack: Some(EsgAckResponse {
				level: Some(value.ack1.level),
				success: Some(value.ack1.success),
				code: value.ack1.code,
				message: value.ack1.message,
			}),
			latest_ack: None,
			status: None,
		}
	}
}

#[cfg(test)]
mod preflight_tests {
	use super::*;

	fn configure() {
		std::env::set_var("AS2_SUBMITTER_URL", "http://127.0.0.1:1");
		std::env::set_var("AS2_SUBMITTER_TOKEN", "isolated-test-token");
		std::env::set_var("AS2_ACK_CALLBACK_URL", "http://127.0.0.1:2/callback");
		std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");
	}
	fn clear() {
		for key in [
			"AS2_SUBMITTER_URL",
			"AS2_SUBMITTER_TOKEN",
			"AS2_ACK_CALLBACK_URL",
			"AS2_SUBMITTER_TIMEOUT_SECS",
		] {
			std::env::remove_var(key);
		}
	}

	#[test]
	fn missing_or_invalid_transport_input_fails_before_a_request_can_be_dispatched()
	{
		for (key, value) in [
			("AS2_SUBMITTER_TIMEOUT_SECS", None),
			("AS2_SUBMITTER_TIMEOUT_SECS", Some("0")),
			("AS2_SUBMITTER_URL", Some("not-a-url")),
			("AS2_SUBMITTER_URL", Some("ftp://example.invalid")),
			("AS2_SUBMITTER_TOKEN", Some("invalid\nheader")),
			("AS2_ACK_CALLBACK_URL", Some("")),
		] {
			configure();
			if let Some(value) = value {
				std::env::set_var(key, value);
			} else {
				std::env::remove_var(key);
			}
			assert!(
				prepare_as2_request(
					Uuid::new_v4(),
					SubmissionAuthority::Fda,
					Some("<xml/>"),
					Some(Uuid::new_v4()),
					Some(1),
					Some("TEST-ROUTE"),
				)
				.is_err(),
				"{key}"
			);
		}
		clear();
	}

	#[tokio::test]
	async fn refused_connection_is_known_not_sent() {
		configure();
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("test listener");
		std::env::set_var(
			"AS2_SUBMITTER_URL",
			format!("http://{}", listener.local_addr().expect("address")),
		);
		let request = prepare_as2_request(
			Uuid::new_v4(),
			SubmissionAuthority::Fda,
			Some("<xml/>"),
			Some(Uuid::new_v4()),
			Some(1),
			Some("TEST-ROUTE"),
		)
		.expect("prepared request");
		drop(listener);
		let failure = request.execute().await.err().expect("connection must fail");
		assert!(failure.not_sent);
		clear();
	}

	#[tokio::test]
	async fn lost_response_after_connection_is_uncertain() {
		use tokio::io::AsyncReadExt;
		configure();
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("test listener");
		std::env::set_var(
			"AS2_SUBMITTER_URL",
			format!("http://{}", listener.local_addr().expect("address")),
		);
		let request = prepare_as2_request(
			Uuid::new_v4(),
			SubmissionAuthority::Fda,
			Some("<xml/>"),
			Some(Uuid::new_v4()),
			Some(1),
			Some("TEST-ROUTE"),
		)
		.expect("prepared request");
		let server = tokio::spawn(async move {
			let (mut stream, _) = listener.accept().await.expect("connection");
			let mut bytes = [0; 4096];
			assert!(stream.read(&mut bytes).await.expect("request bytes") > 0);
		});
		let failure = request
			.execute()
			.await
			.err()
			.expect("missing response must fail");
		assert!(!failure.not_sent);
		server.await.expect("test server");
		clear();
	}
}
