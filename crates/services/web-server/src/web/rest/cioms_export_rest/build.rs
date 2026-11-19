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
		ordered.test_results.reverse();
		ordered.primary_sources.reverse();
		ordered.senders.reverse();
		ordered.causality_rows.reverse();
		ordered.medical_history_episodes.reverse();
		ordered.past_drug_history.reverse();
	}
	ordered
}

pub(super) fn build_cioms_pdf_with_options(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
) -> Result<Vec<u8>> {
	let (width, height) = if settings.orientation.eq_ignore_ascii_case("Portrait") {
		(595, 842)
	} else {
		(
			CIOMS_LANDSCAPE_TEMPLATE.page_width,
			CIOMS_LANDSCAPE_TEMPLATE.page_height,
		)
	};
	let ordered = ordered_cioms_case_data(data, settings);
	let mut canvas = PdfCanvas::with_font_mapping(&[])?;
	canvas.stream.push_str("0.8 w\n");
	render_cioms_first_page(
		&mut canvas,
		&ordered,
		settings,
		options,
		width,
		height,
	)?;
	let mut font_mapping = canvas.font_mapping();
	let first_stream = canvas.stream;
	let overflow = collect_cioms_overflow(&ordered, settings, options)?;
	let continuation_streams =
		if cioms_continuation_required(&ordered, settings, &overflow) {
			let (pages, mapping) = render_cioms_continuation_pages(
				&ordered,
				settings,
				options,
				&overflow,
				width,
				height,
				&font_mapping,
			)?;
			font_mapping = mapping;
			pages
		} else {
			Vec::new()
		};

	let obj1 = "<< /Type /Catalog /Pages 2 0 R >>";
	let page_refs = std::iter::once(3)
		.chain((0..continuation_streams.len()).map(|index| 12 + (index * 2)))
		.map(|page| format!("{page} 0 R"))
		.collect::<Vec<_>>()
		.join(" ");
	let page_count = 1 + continuation_streams.len();
	let obj2 = format!("<< /Type /Pages /Kids [{page_refs}] /Count {page_count} >>");
	let obj3 = format!(
		"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 8 0 R >>"
	);
	let obj4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
	let obj5 = format!(
		"<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding 9 0 R /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",
		font_name()
	);
	let obj6 = format!(
		"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 10 0 R /DW 1000 /W {} >>",
		font_name(),
		font_widths()?,
	);
	let to_unicode = to_unicode_cmap(&font_mapping)?;
	let obj7 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		to_unicode.len(),
		to_unicode
	);
	let obj8 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		first_stream.len(),
		first_stream
	);
	let encoding = encoding_cmap(&font_mapping)?;
	let obj9 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		encoding.len(),
		encoding
	);
	let obj10 = format!(
		"<< /Type /FontDescriptor /FontName /{} /Flags 32 /FontBBox [-1002 -1048 2928 1808] /ItalicAngle 0 /Ascent 1160 /Descent -288 /CapHeight 733 /StemV 80 /FontFile3 11 0 R >>",
		font_name()
	);
	let compressed_font = compressed_font()?;
	let mut obj11 = format!(
		"<< /Length {} /Subtype /CIDFontType0C /Filter /FlateDecode >>\nstream\n",
		compressed_font.len()
	)
	.into_bytes();
	obj11.extend_from_slice(compressed_font);
	obj11.extend_from_slice(b"\nendstream");
	let mut objects = vec![
		obj1.as_bytes().to_vec(),
		obj2.into_bytes(),
		obj3.into_bytes(),
		obj4.as_bytes().to_vec(),
		obj5.into_bytes(),
		obj6.into_bytes(),
		obj7.into_bytes(),
		obj8.into_bytes(),
		obj9.into_bytes(),
		obj10.into_bytes(),
		obj11,
	];
	let mut page_object = 12;
	for stream in continuation_streams {
		objects.push(format!(
			"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents {} 0 R >>",
			page_object + 1
		).into_bytes());
		objects.push(
			format!(
				"<< /Length {} >>\nstream\n{}endstream",
				stream.len(),
				stream
			)
			.into_bytes(),
		);
		page_object += 2;
	}

	let mut pdf = b"%PDF-1.4\n".to_vec();
	let mut offsets = Vec::with_capacity(objects.len());
	for (idx, object) in objects.iter().enumerate() {
		offsets.push(pdf.len());
		pdf.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
		pdf.extend_from_slice(object);
		pdf.extend_from_slice(b"\nendobj\n");
	}
	let xref_offset = pdf.len();
	pdf.extend_from_slice(b"xref\n");
	pdf.extend_from_slice(format!("0 {}\n", objects.len() + 1).as_bytes());
	pdf.extend_from_slice(b"0000000000 65535 f \n");
	for offset in offsets {
		pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
	}
	pdf.extend_from_slice(
		format!(
			"trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
			objects.len() + 1
		)
		.as_bytes(),
	);
	Ok(pdf)
}

pub async fn export_case_cioms_pdf(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Query(query): Query<ExportCiomsQuery>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_export(
		&ctx,
		&snapshot,
		&mm,
		&[id],
		move |ctx, mm| {
			Box::pin(async move {
				let settings = load_cioms_settings(ctx, mm).await?;
				let data = load_cioms_case_data(ctx, mm, id).await?;
				let pdf = build_cioms_pdf_with_options(
					&data,
					&settings,
					CiomsExportOptions {
						include_notation: match query.include_notation {
							Some(value) => value,
							None => settings.notation,
						},
					},
				)?;
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
			})
		},
	)
	.await
}
