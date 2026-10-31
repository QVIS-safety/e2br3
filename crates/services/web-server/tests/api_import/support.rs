use crate::common::Result;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

pub(super) fn scenario6(root: &std::path::Path) -> Result<String> {
	Ok(std::fs::read_to_string(
		root.join("docs/exporter/fda/FAERS2022Scenario6.xml"),
	)?
	.replace("201411011202", "2014110112")
	.replace("201401011041", "2014010110")
	.replace("20220614101010-0500", "20140614151617-0500")
	.replace(
		"<value xsi:type=\"CE\" code=\"10060051\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"/>",
		"<value xsi:type=\"CE\" code=\"10060051\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"><originalText>Reported reaction</originalText></value>",
	)
	.replace(
		"<value xsi:type=\"CE\" code=\"10024381\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"/>",
		"<value xsi:type=\"CE\" code=\"10024381\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"><originalText>Reported reaction</originalText></value>",
	)
	.replace(
		"<code code=\"10005362\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"/>",
		"<code code=\"10005362\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"><originalText>Test result</originalText></code>",
	)
	.replace(
		"<code code=\"10062994\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"/>",
		"<code code=\"10062994\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"12.0\"><originalText>Test result</originalText></code>",
	)
	.replace("\n.\n\n\t\t\t\t\t\t\t</text>", "\n\n\t\t\t\t\t\t\t</text>"))
}

pub(super) async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body).map_err(|err| {
		format!(
			"invalid JSON response ({status}): {err}; body={}",
			String::from_utf8_lossy(&body)
		)
	})?;
	Ok((status, value))
}

pub(super) async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body).map_err(|err| {
		format!(
			"invalid JSON response ({status}): {err}; body={}",
			String::from_utf8_lossy(&body)
		)
	})?;
	Ok((status, value))
}

pub(super) async fn import_xml_fixture_with_product(
	app: &axum::Router,
	cookie: &str,
	filename: &str,
	xml: &[u8],
	product_presave_id: &str,
) -> Result<(StatusCode, Value)> {
	let boundary = "X-BOUNDARY-IMPORT-PRODUCT";
	let mut multipart = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{product_presave_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n"
	)
	.into_bytes();
	multipart.extend_from_slice(xml);
	multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", cookie)
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(multipart))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

pub(super) async fn create_product_presave(
	app: &axum::Router,
	cookie: &str,
	sender_presave_id: &str,
	product_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		"/api/presaves/products",
		json!({
			"data": { "rows": {
				"product": {
					"senderPresaveId": sender_presave_id,
					"productId": product_id,
					"medicinalProduct": product_id
				},
				"activeSubstances": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	body["data"]["rows"]["product"]["id"]
		.as_str()
		.map(str::to_string)
		.ok_or_else(|| format!("missing product id in body {body:?}").into())
}
