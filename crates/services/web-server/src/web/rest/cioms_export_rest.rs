use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use lib_core::model::acs::XML_EXPORT;
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::CaseBmc;
use lib_core::model::ModelManager;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

const SETTINGS_KEY: &str = "system";

#[derive(Debug, Clone)]
struct CiomsSettings {
	orientation: String,
	data_ordering: String,
}

async fn load_cioms_settings(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
) -> Result<CiomsSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	let orientation = value
		.as_ref()
		.and_then(|value| value.get("orientation"))
		.and_then(|value| value.as_str())
		.unwrap_or("Landscape")
		.trim()
		.to_string();
	let data_ordering = value
		.as_ref()
		.and_then(|value| value.get("data_ordering"))
		.and_then(|value| value.as_str())
		.unwrap_or("Primary data will appear first")
		.trim()
		.to_string();
	Ok(CiomsSettings {
		orientation: if orientation.eq_ignore_ascii_case("portrait") {
			"Portrait".to_string()
		} else {
			"Landscape".to_string()
		},
		data_ordering,
	})
}

fn escape_pdf_text(value: &str) -> String {
	value
		.chars()
		.flat_map(|ch| match ch {
			'(' => "\\(".chars().collect::<Vec<_>>(),
			')' => "\\)".chars().collect::<Vec<_>>(),
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			'\n' | '\r' => " ".chars().collect::<Vec<_>>(),
			_ => vec![ch],
		})
		.collect()
}

fn build_cioms_pdf(case_number: &str, settings: &CiomsSettings) -> Vec<u8> {
	let (width, height) = if settings.orientation == "Portrait" {
		(595, 842)
	} else {
		(842, 595)
	};
	let stream = format!(
		"BT\n/F1 18 Tf\n50 {} Td\n(CIOMS I Safety Report) Tj\n0 -28 Td\n(Case: {}) Tj\n0 -22 Td\n(Orientation: {}) Tj\n0 -22 Td\n(Data ordering: {}) Tj\nET\n",
		height - 70,
		escape_pdf_text(case_number),
		escape_pdf_text(&settings.orientation),
		escape_pdf_text(&settings.data_ordering),
	);
	let obj1 = "<< /Type /Catalog /Pages 2 0 R >>";
	let obj2 = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
	let obj3 = format!(
		"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
	);
	let obj4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
	let obj5 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		stream.len(),
		stream
	);
	let objects = [
		obj1.to_string(),
		obj2.to_string(),
		obj3,
		obj4.to_string(),
		obj5,
	];

	let mut pdf = String::from("%PDF-1.4\n");
	let mut offsets = Vec::with_capacity(objects.len());
	for (idx, object) in objects.iter().enumerate() {
		offsets.push(pdf.len());
		pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", idx + 1, object));
	}
	let xref_offset = pdf.len();
	pdf.push_str("xref\n");
	pdf.push_str(&format!("0 {}\n", objects.len() + 1));
	pdf.push_str("0000000000 65535 f \n");
	for offset in offsets {
		pdf.push_str(&format!("{offset:010} 00000 n \n"));
	}
	pdf.push_str(&format!(
		"trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
		objects.len() + 1
	));
	pdf.into_bytes()
}

pub async fn export_case_cioms_pdf(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let case = CaseBmc::get(&ctx, &mm, id).await.map_err(Error::Model)?;
	let settings = load_cioms_settings(&ctx, &mm).await?;
	let pdf = build_cioms_pdf(&case.safety_report_id, &settings);
	let file_name = format!("{}-cioms.pdf", case.safety_report_id);

	let mut response = (StatusCode::OK, pdf).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("application/pdf"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{file_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid CIOMS filename header: {err}"),
		})?,
	);
	Ok(response)
}
