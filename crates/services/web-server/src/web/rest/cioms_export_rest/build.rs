use super::*;

pub(super) fn ordered_cioms_case_data(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
) -> CiomsCaseData {
	let mut ordered = data.clone();
	if settings
		.data_ordering
		.eq_ignore_ascii_case("Latest data will appear first")
	{
		ordered.reactions.reverse();
		ordered.drugs.reverse();
		ordered.dosages.reverse();
		ordered.indications.reverse();
		ordered.primary_sources.reverse();
		ordered.senders.reverse();
	}
	ordered
}

pub(super) fn build_cioms_pdf(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
) -> Vec<u8> {
	build_cioms_pdf_with_options(data, settings, CiomsExportOptions::default())
}

pub(super) fn build_cioms_pdf_with_options(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
) -> Vec<u8> {
	let (width, height) = if settings.orientation == "Portrait" {
		(595, 842)
	} else {
		(
			CIOMS_LANDSCAPE_TEMPLATE.page_width,
			CIOMS_LANDSCAPE_TEMPLATE.page_height,
		)
	};
	let ordered = ordered_cioms_case_data(data, settings);
	let mut canvas = PdfCanvas::new();
	canvas.stream.push_str("0.8 w\n");
	if settings.orientation == "Portrait" {
		render_landscape_cioms_on_portrait_page(
			&mut canvas,
			&ordered,
			settings,
			options,
			width,
			height,
		);
	} else {
		render_landscape_cioms(
			&mut canvas,
			&ordered,
			settings,
			options,
			width,
			height,
		);
	}
	let first_stream = canvas.stream;
	let overflow = collect_cioms_overflow(&ordered, settings, options);
	let continuation_stream = if overflow.is_empty() {
		None
	} else {
		let mut continuation = PdfCanvas::new();
		render_cioms_continuation_page(
			&mut continuation,
			&ordered.case_number,
			&overflow,
			width,
			height,
		);
		Some(continuation.stream)
	};

	let obj1 = "<< /Type /Catalog /Pages 2 0 R >>";
	let page_count = if continuation_stream.is_some() { 2 } else { 1 };
	let page_kids = if continuation_stream.is_some() {
		"[3 0 R 6 0 R]"
	} else {
		"[3 0 R]"
	};
	let obj2 = format!("<< /Type /Pages /Kids {page_kids} /Count {page_count} >>");
	let obj3 = format!(
		"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
	);
	let obj4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
	let obj5 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		first_stream.len(),
		first_stream
	);
	let mut objects = vec![obj1.to_string(), obj2, obj3, obj4.to_string(), obj5];
	if let Some(stream) = continuation_stream {
		objects.push(format!(
			"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>"
		));
		objects.push(format!(
			"<< /Length {} >>\nstream\n{}endstream",
			stream.len(),
			stream
		));
	}

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
	Query(query): Query<ExportCiomsQuery>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let settings = load_cioms_settings(&ctx, &mm).await?;
	let data = load_cioms_case_data(&ctx, &mm, id).await?;
	let pdf = build_cioms_pdf_with_options(
		&data,
		&settings,
		CiomsExportOptions {
			include_notation: query.include_notation.unwrap_or(false),
		},
	);
	let file_name = format!("{}-cioms.pdf", data.case_number);

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
