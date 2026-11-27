use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{Error, ModelManager, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

const MAX_PKCS12_BYTES: usize = 1024 * 1024;
const MAX_PARTNER_CERT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationAs2Profile {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub authority: String,
	pub endpoint_url: String,
	pub as2_to: String,
	pub expected_ack_as2_from: String,
	pub extra_headers: Value,
	pub signing_key_alias: Option<String>,
	pub async_mdn_url: Option<String>,
	pub active: bool,
	pub version: i64,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationAs2ProfileInput {
	pub endpoint_url: String,
	pub as2_to: String,
	pub expected_ack_as2_from: String,
	#[serde(default = "empty_object")]
	pub extra_headers: Value,
	pub signing_pkcs12_base64: String,
	pub signing_pkcs12_password: String,
	pub signing_key_alias: Option<String>,
	pub partner_certificate_pem: String,
	pub async_mdn_url: Option<String>,
}

#[derive(Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct As2RuntimeProfile {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub authority: String,
	pub routing_id: String,
	pub endpoint_url: String,
	pub as2_to: String,
	pub expected_ack_as2_from: String,
	pub extra_headers: Value,
	pub signing_pkcs12_base64: String,
	pub signing_pkcs12_password: String,
	pub signing_key_alias: Option<String>,
	pub partner_certificate_pem: String,
	pub async_mdn_url: Option<String>,
	pub version: i64,
}

fn empty_object() -> Value {
	serde_json::json!({})
}

fn encryption_key() -> Result<String> {
	let key = std::env::var("AS2_PROFILE_ENCRYPTION_KEY").map_err(|_| {
		Error::Validation {
			message: "AS2_PROFILE_ENCRYPTION_KEY is required".to_string(),
		}
	})?;
	if key.len() < 32 {
		return Err(Error::Validation {
			message:
				"AS2_PROFILE_ENCRYPTION_KEY must contain at least 32 characters"
					.to_string(),
		});
	}
	Ok(key)
}

fn required(value: &str, field: &str) -> Result<String> {
	let value = value.trim();
	if value.is_empty() {
		return Err(Error::Validation {
			message: format!("{field} is required"),
		});
	}
	Ok(value.to_string())
}

fn validate_url(value: &str, field: &str) -> Result<String> {
	let value = required(value, field)?;
	let url = reqwest::Url::parse(&value).map_err(|_| Error::Validation {
		message: format!("{field} must be an absolute HTTP(S) URL"),
	})?;
	let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
	if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
		return Err(Error::Validation {
			message: format!(
				"{field} must use HTTPS except for a literal loopback host"
			),
		});
	}
	if url.host_str().is_none()
		|| !url.username().is_empty()
		|| url.password().is_some()
		|| url.fragment().is_some()
	{
		return Err(Error::Validation {
			message: format!(
				"{field} must have a host and no credentials or fragment"
			),
		});
	}
	Ok(value)
}

fn normalize_authority(authority: &str) -> Result<String> {
	let authority = authority.trim().to_ascii_lowercase();
	if !matches!(authority.as_str(), "fda" | "mfds") {
		return Err(Error::Validation {
			message: "authority must be fda or mfds".to_string(),
		});
	}
	Ok(authority)
}

fn validated_input(
	input: OrganizationAs2ProfileInput,
) -> Result<(OrganizationAs2ProfileInput, Vec<u8>)> {
	let pkcs12 = STANDARD
		.decode(input.signing_pkcs12_base64.trim())
		.map_err(|_| Error::Validation {
			message: "signingPkcs12Base64 must be valid base64".to_string(),
		})?;
	if pkcs12.is_empty() || pkcs12.len() > MAX_PKCS12_BYTES {
		return Err(Error::Validation {
			message: format!(
				"signing PKCS#12 must contain 1 to {MAX_PKCS12_BYTES} bytes"
			),
		});
	}
	if input.signing_pkcs12_password.is_empty() {
		return Err(Error::Validation {
			message: "signingPkcs12Password is required".to_string(),
		});
	}
	let signing = openssl::pkcs12::Pkcs12::from_der(&pkcs12)
		.and_then(|store| store.parse2(&input.signing_pkcs12_password))
		.map_err(|_| Error::Validation {
			message: "invalid PKCS#12 or certificate password".into(),
		})?;
	let (Some(key), Some(certificate)) = (signing.pkey, signing.cert) else {
		return Err(Error::Validation {
			message: "PKCS#12 must contain a private key and certificate".into(),
		});
	};
	let public = certificate.public_key().map_err(|_| Error::Validation {
		message: "invalid signing certificate".into(),
	})?;
	if !key.public_eq(&public) {
		return Err(Error::Validation {
			message: "certificate and private key do not match".into(),
		});
	}
	if let Some(alias) = input
		.signing_key_alias
		.as_deref()
		.filter(|s| !s.trim().is_empty())
	{
		if !certificate.alias().is_some_and(|actual| {
			actual.eq_ignore_ascii_case(alias.trim().as_bytes())
		}) {
			return Err(Error::Validation {
				message: "signingKeyAlias does not match the PKCS#12 identity"
					.into(),
			});
		}
	}
	let cert = input.partner_certificate_pem.trim();
	if cert.len() > MAX_PARTNER_CERT_BYTES
		|| !cert.contains("-----BEGIN CERTIFICATE-----")
		|| !cert.contains("-----END CERTIFICATE-----")
	{
		return Err(Error::Validation {
			message: "partnerCertificatePem must contain one PEM certificate"
				.to_string(),
		});
	}
	let partner_certs = openssl::x509::X509::stack_from_pem(cert.as_bytes())
		.map_err(|_| Error::Validation {
			message: "partnerCertificatePem must contain one valid PEM certificate"
				.into(),
		})?;
	if partner_certs.len() != 1 {
		return Err(Error::Validation {
			message: "partnerCertificatePem must contain exactly one certificate"
				.into(),
		});
	}
	if !input
		.extra_headers
		.as_object()
		.is_some_and(|headers| headers.values().all(Value::is_string))
	{
		return Err(Error::Validation {
			message: "extraHeaders must be an object containing string values"
				.to_string(),
		});
	}
	let headers = input.extra_headers.as_object().ok_or(Error::Validation {
		message: "extraHeaders must be an object".into(),
	})?;
	for (name, value) in headers {
		if !matches!(
			name.as_str(),
			"X-Cyclone-True-Receiver"
				| "X-Cyclone-Metadata-FdaCenter"
				| "X-Cyclone-Metadata-FdaSubmissionType"
		) || value.as_str().is_some_and(|s| s.contains(['\r', '\n']))
		{
			return Err(Error::Validation {
				message: format!("unsupported AS2 extra header: {name}"),
			});
		}
	}
	for value in [&input.as2_to, &input.expected_ack_as2_from] {
		if value.len() > 255 || value.contains(['\r', '\n', '"']) {
			return Err(Error::Validation {
				message:
					"AS2 IDs must be at most 255 bytes without quotes or line breaks"
						.into(),
			});
		}
	}
	Ok((
		OrganizationAs2ProfileInput {
			endpoint_url: validate_url(&input.endpoint_url, "endpointUrl")?,
			as2_to: required(&input.as2_to, "as2To")?,
			expected_ack_as2_from: required(
				&input.expected_ack_as2_from,
				"expectedAckAs2From",
			)?,
			extra_headers: input.extra_headers,
			signing_pkcs12_base64: input.signing_pkcs12_base64,
			signing_pkcs12_password: input.signing_pkcs12_password,
			signing_key_alias: input
				.signing_key_alias
				.map(|value| value.trim().to_string())
				.filter(|value| !value.is_empty()),
			partner_certificate_pem: cert.to_string(),
			async_mdn_url: input
				.async_mdn_url
				.map(|value| validate_url(&value, "asyncMdnUrl"))
				.transpose()?,
		},
		pkcs12,
	))
}

pub struct OrganizationAs2ProfileBmc;

async fn begin(ctx: &Ctx, mm: &ModelManager) -> Result<()> {
	mm.dbx().begin_txn().await?;
	if let Err(error) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		mm.dbx().rollback_txn().await?;
		return Err(error);
	}
	Ok(())
}

async fn finish<T>(mm: &ModelManager, result: Result<T>) -> Result<T> {
	match result {
		Ok(value) => {
			mm.dbx().commit_txn().await?;
			Ok(value)
		}
		Err(error) => {
			mm.dbx().rollback_txn().await?;
			Err(error)
		}
	}
}

impl OrganizationAs2ProfileBmc {
	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
	) -> Result<Vec<OrganizationAs2Profile>> {
		begin(ctx, mm).await?;
		let result = mm
			.dbx()
			.fetch_all(
				sqlx::query_as(
					"SELECT id,organization_id,authority,endpoint_url,as2_to,
				 expected_ack_as2_from,extra_headers,signing_key_alias,async_mdn_url,
				 active,version,created_at,updated_at
				 FROM organization_as2_profiles WHERE organization_id=$1
				 ORDER BY authority",
				)
				.bind(organization_id),
			)
			.await
			.map_err(Error::from);
		finish(mm, result).await
	}

	pub async fn upsert(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
		authority: &str,
		input: OrganizationAs2ProfileInput,
	) -> Result<OrganizationAs2Profile> {
		let authority = normalize_authority(authority)?;
		let (input, pkcs12) = validated_input(input)?;
		let key = encryption_key()?;
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(error) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(error);
		}
		let result = async {
			dbx.execute(
				sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
					.bind(format!("as2:{organization_id}:{authority}")),
			)
			.await?;
			let row = dbx
				.fetch_one(
					sqlx::query_as::<_, OrganizationAs2Profile>(
						"INSERT INTO organization_as2_profiles
				 (organization_id,authority,endpoint_url,as2_to,
				  expected_ack_as2_from,extra_headers,signing_pkcs12_ciphertext,
				  signing_password_ciphertext,signing_key_alias,partner_certificate_pem,
				  async_mdn_url,active,created_by,updated_by)
				 VALUES($1,$2,$3,$4,$5,$6,
				  pgp_sym_encrypt_bytea($7,$8,'cipher-algo=aes256,compress-algo=0'),
				  pgp_sym_encrypt($9,$8,'cipher-algo=aes256,compress-algo=0'),
				  $10,$11,$12,true,$13,$13)
				 ON CONFLICT (organization_id,authority) DO UPDATE SET
				  endpoint_url=EXCLUDED.endpoint_url,as2_to=EXCLUDED.as2_to,
				  expected_ack_as2_from=EXCLUDED.expected_ack_as2_from,
				  extra_headers=EXCLUDED.extra_headers,
				  signing_pkcs12_ciphertext=EXCLUDED.signing_pkcs12_ciphertext,
				  signing_password_ciphertext=EXCLUDED.signing_password_ciphertext,
				  signing_key_alias=EXCLUDED.signing_key_alias,
				  partner_certificate_pem=EXCLUDED.partner_certificate_pem,
				  async_mdn_url=EXCLUDED.async_mdn_url,active=true,
				  version=organization_as2_profiles.version+1,updated_by=EXCLUDED.updated_by
				 RETURNING id,organization_id,authority,endpoint_url,as2_to,
				  expected_ack_as2_from,extra_headers,signing_key_alias,async_mdn_url,
				  active,version,created_at,updated_at",
					)
					.bind(organization_id)
					.bind(&authority)
					.bind(&input.endpoint_url)
					.bind(&input.as2_to)
					.bind(&input.expected_ack_as2_from)
					.bind(&input.extra_headers)
					.bind(pkcs12)
					.bind(&key)
					.bind(&input.signing_pkcs12_password)
					.bind(&input.signing_key_alias)
					.bind(&input.partner_certificate_pem)
					.bind(&input.async_mdn_url)
					.bind(ctx.user_id()),
				)
				.await?;
			Ok::<_, crate::model::store::dbx::Error>(row)
		}
		.await;
		match result {
			Ok(row) => {
				dbx.commit_txn().await?;
				Ok(row)
			}
			Err(error) => {
				dbx.rollback_txn().await?;
				Err(Error::from(error))
			}
		}
	}

	pub async fn deactivate(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
		authority: &str,
	) -> Result<()> {
		let authority = normalize_authority(authority)?;
		begin(ctx, mm).await?;
		let result = async {
			let changed = mm
			.dbx()
			.execute(
				sqlx::query(
					"UPDATE organization_as2_profiles SET active=false,updated_by=$3
			 WHERE organization_id=$1 AND authority=$2 AND active",
				)
				.bind(organization_id)
				.bind(authority)
				.bind(ctx.user_id()),
			)
			.await?;
			if changed == 0 {
				return Err(Error::EntityUuidNotFound {
					entity: "organization_as2_profiles",
					id: organization_id,
				});
			}
			Ok(())
		}
		.await;
		finish(mm, result).await
	}

	pub async fn active_for_submission(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
		authority: &str,
	) -> Result<OrganizationAs2Profile> {
		let authority = normalize_authority(authority)?;
		begin(ctx, mm).await?;
		let result = async { mm.dbx().fetch_optional(sqlx::query_as(
			"SELECT id,organization_id,authority,endpoint_url,as2_to,
			 expected_ack_as2_from,extra_headers,signing_key_alias,async_mdn_url,
			 active,version,created_at,updated_at
			 FROM organization_as2_profiles
			 WHERE organization_id=$1 AND authority=$2 AND active",
		).bind(organization_id).bind(&authority)).await?
		.ok_or(Error::Validation {
			message: format!("organization {organization_id} has no active AS2 profile for {authority}"),
		}) }.await;
		finish(mm, result).await
	}

	pub async fn runtime_profiles(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<Vec<As2RuntimeProfile>> {
		let key = encryption_key()?;
		begin(ctx, mm).await?;
		let result = mm.dbx().fetch_all(sqlx::query_as(
			"SELECT p.id,p.organization_id,p.authority,g.routing_id,p.endpoint_url,p.as2_to,
			 p.expected_ack_as2_from,p.extra_headers,
			 encode(pgp_sym_decrypt_bytea(p.signing_pkcs12_ciphertext,$1),'base64') AS signing_pkcs12_base64,
			 pgp_sym_decrypt(p.signing_password_ciphertext,$1) AS signing_pkcs12_password,
			 p.signing_key_alias,p.partner_certificate_pem,p.async_mdn_url,p.version
			 FROM organization_as2_profiles p
			 JOIN LATERAL (
			   SELECT DISTINCT btrim(gateway.routing_identifier) AS routing_id
			   FROM sender_presave_gateways gateway
			   JOIN sender_presaves sender ON sender.id=gateway.sender_presave_id
			   WHERE sender.organization_id=p.organization_id AND NOT sender.deleted
			     AND NOT gateway.deleted AND gateway.is_default_for_authority
			     AND lower(btrim(gateway.gateway_authority))=p.authority
			     AND btrim(gateway.routing_identifier) <> ''
			 ) g ON true
			 WHERE p.active ORDER BY p.organization_id,p.authority,g.routing_id",
		).bind(key)).await.map_err(Error::from);
		finish(mm, result).await
	}
}
