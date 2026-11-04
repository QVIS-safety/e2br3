use super::helpers as c_helpers;
use crate::import::CImportSettings;
use crate::import_sections::c_safety_report::CSafetyReportImport;
use crate::import_sections::shared;
use crate::{error::Error, Result};
use lib_core::ctx::Ctx;
use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberForCreate, LinkedReportNumberForUpdate,
	OtherCaseIdentifierBmc, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use lib_core::model::presave::{
	SenderPresave, SenderPresaveBmc, SenderPresaveResponsiblePersonBmc,
};
use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use lib_core::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderForCreate,
	DocumentsHeldBySenderForUpdate, LiteratureReferenceBmc,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySourceBmc,
	PrimarySourceForCreate, PrimarySourceForUpdate, SenderInformationBmc,
	SenderInformationForCreate, SenderInformationForUpdate,
};
use lib_core::model::{self, ModelManager};
use sqlx::types::time::Date;
use sqlx::types::Uuid;

pub fn apply_c_safety_report_import_settings(
	report: &mut CSafetyReportImport,
	settings: &CImportSettings,
	import_date: Date,
) -> Result<()> {
	if settings.update_date_of_creation {
		report.transmission_date = Some(format_e2b_datetime(import_date));
	}
	if settings.update_most_recent_info_date {
		report.date_of_most_recent_information = Some(format_e2b_date(import_date));
	}
	if settings.update_report_first_received_date {
		report.date_first_received_from_source = Some(format_e2b_date(import_date));
	}
	Ok(())
}

fn format_e2b_datetime(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}000000",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn format_e2b_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

pub(crate) async fn import_section_c(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	safety_report_id: &str,
	version: i32,
	header: Option<&shared::MessageHeaderExtract>,
	settings: &CImportSettings,
) -> Result<()> {
	import_c_1_safety_report(
		ctx,
		mm,
		xml,
		case_id,
		safety_report_id,
		version,
		header,
		settings,
	)
	.await?;
	import_c_2_sender_information(ctx, mm, xml, case_id, settings).await?;
	import_c_3_primary_sources(ctx, mm, xml, case_id).await?;
	import_c_4_case_identifiers(ctx, mm, xml, case_id).await?;
	import_c_4_documents_held_by_sender(ctx, mm, xml, case_id).await?;
	import_c_4_literature_references(ctx, mm, xml, case_id).await?;
	import_c_5_study_information(ctx, mm, xml, case_id).await?;
	import_c_6_receiver_information(ctx, mm, xml, case_id).await?;
	Ok(())
}

async fn import_c_1_safety_report(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	safety_report_id: &str,
	version: i32,
	header: Option<&shared::MessageHeaderExtract>,
	settings: &CImportSettings,
) -> Result<()> {
	let mut report =
		crate::import_sections::c_safety_report::parse_c_safety_report(xml)?
			.ok_or_else(|| Error::InvalidImportRequest {
				message: "C.1 safety report section missing".to_string(),
			})?;
	apply_c_safety_report_import_settings(
		&mut report,
		settings,
		settings
			.import_date
			.unwrap_or_else(|| time::OffsetDateTime::now_utc().date()),
	)?;

	let receiver_organization = header.and_then(|h| h.message_receiver.clone());

	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO safety_report_identification (
					case_id,
					safety_report_id,
					version,
					transmission_date,
					report_type,
					date_first_received_from_source,
					date_of_most_recent_information,
					fulfil_expedited_criteria,
					fulfil_expedited_criteria_null_flavor,
					local_criteria_report_type,
					combination_product_report_indicator,
					combination_product_report_indicator_null_flavor,
					worldwide_unique_id,
					first_sender_type,
					additional_documents_available,
					other_case_identifiers_exist,
					other_case_identifiers_exist_null_flavor,
					nullification_code,
					nullification_reason,
					receiver_organization,
					created_at,
					updated_at,
					created_by
				) VALUES (
					$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,NOW(),NOW(),$21
				)
				ON CONFLICT (case_id) DO UPDATE SET
					safety_report_id = EXCLUDED.safety_report_id,
					version = EXCLUDED.version,
					transmission_date = EXCLUDED.transmission_date,
					report_type = EXCLUDED.report_type,
					date_first_received_from_source = EXCLUDED.date_first_received_from_source,
					date_of_most_recent_information = EXCLUDED.date_of_most_recent_information,
					fulfil_expedited_criteria = EXCLUDED.fulfil_expedited_criteria,
					fulfil_expedited_criteria_null_flavor = EXCLUDED.fulfil_expedited_criteria_null_flavor,
					local_criteria_report_type = EXCLUDED.local_criteria_report_type,
					combination_product_report_indicator = EXCLUDED.combination_product_report_indicator,
					combination_product_report_indicator_null_flavor = EXCLUDED.combination_product_report_indicator_null_flavor,
					worldwide_unique_id = EXCLUDED.worldwide_unique_id,
					first_sender_type = EXCLUDED.first_sender_type,
					additional_documents_available = EXCLUDED.additional_documents_available,
					other_case_identifiers_exist = EXCLUDED.other_case_identifiers_exist,
					other_case_identifiers_exist_null_flavor = EXCLUDED.other_case_identifiers_exist_null_flavor,
					nullification_code = EXCLUDED.nullification_code,
					nullification_reason = EXCLUDED.nullification_reason,
					receiver_organization = EXCLUDED.receiver_organization,
					updated_at = NOW(),
					updated_by = $21",
			)
			.bind(case_id)
			.bind(safety_report_id)
			.bind(version)
			.bind(report.transmission_date)
			.bind(report.report_type)
			.bind(report.date_first_received_from_source)
			.bind(report.date_of_most_recent_information)
			.bind(report.fulfil_expedited_criteria)
			.bind(report.fulfil_expedited_criteria_null_flavor)
			.bind(report.local_criteria_report_type)
			.bind(report.combination_product_report_indicator)
			.bind(report.combination_product_report_indicator_null_flavor)
			.bind(report.worldwide_unique_id)
			.bind(report.first_sender_type)
			.bind(report.additional_documents_available)
			.bind(report.other_case_identifiers_exist)
			.bind(report.other_case_identifiers_exist_null_flavor)
			.bind(report.nullification_code)
			.bind(report.nullification_reason)
			.bind(receiver_organization)
			.bind(ctx.user_id()),
		)
		.await
		.map_err(model::Error::from)?;
	let (visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM safety_report_identification WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?;
	if visible_count != 1 {
		return Err(Error::Model(model::Error::Store(format!(
			"section C safety report write invariant failed for case {case_id}: visible_count={visible_count}"
		))));
	}
	Ok(())
}

async fn import_c_2_sender_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	settings: &CImportSettings,
) -> Result<()> {
	let (sender, source_sender_presave_id) = if settings
		.apply_sender_info_to_imported_cases
	{
		let sender_id = settings.selected_sender_presave_id.ok_or_else(|| {
			Error::InvalidImportRequest {
				message:
					"selected Sender Presave is required when sender application is enabled"
						.to_string(),
			}
		})?;
		let sender_presave = SenderPresaveBmc::get(ctx, mm, sender_id)
			.await
			.map_err(Error::Model)?;
		if sender_presave.deleted {
			return Err(Error::InvalidXml {
				message: "selected Product Sender is deleted".to_string(),
				line: None,
				column: None,
			});
		}
		(
			Some(sender_import_from_presave(ctx, mm, sender_presave).await?),
			Some(sender_id),
		)
	} else {
		(
			c_helpers::parse_sender_information(xml)?,
			settings.selected_sender_presave_id,
		)
	};
	let Some(sender) = sender else {
		return Ok(());
	};

	let sender_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM sender_information WHERE case_id = $1 LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		SenderInformationBmc::create(
			ctx,
			mm,
			SenderInformationForCreate {
				case_id,
				source_sender_presave_id,
				sender_type: sender.sender_type.clone(),
				health_professional_type_kr1: sender
					.health_professional_type_kr1
					.clone(),
				organization_name: sender.organization_name.clone(),
				department: sender.department.clone(),
				street_address: sender.street_address.clone(),
				city: sender.city.clone(),
				state: sender.state.clone(),
				postcode: sender.postcode.clone(),
				country_code: sender.country_code.clone(),
				person_title: sender.person_title.clone(),
				person_title_null_flavor: sender.person_title_null_flavor.clone(),
				person_given_name: sender.person_given_name.clone(),
				person_given_name_null_flavor: sender
					.person_given_name_null_flavor
					.clone(),
				person_middle_name: sender.person_middle_name.clone(),
				person_middle_name_null_flavor: sender
					.person_middle_name_null_flavor
					.clone(),
				person_family_name: sender.person_family_name.clone(),
				person_family_name_null_flavor: sender
					.person_family_name_null_flavor
					.clone(),
				telephone: sender.telephone.clone(),
				fax: sender.fax.clone(),
				email: sender.email.clone(),
			},
		)
		.await?
	};

	SenderInformationBmc::update(
		ctx,
		mm,
		sender_id,
		SenderInformationForUpdate {
			source_sender_presave_id,
			sender_type: sender.sender_type,
			health_professional_type_kr1: sender.health_professional_type_kr1,
			organization_name: sender.organization_name,
			department: sender.department,
			street_address: sender.street_address,
			city: sender.city,
			state: sender.state,
			postcode: sender.postcode,
			country_code: sender.country_code,
			person_title: sender.person_title,
			person_title_null_flavor: sender.person_title_null_flavor,
			person_given_name: sender.person_given_name,
			person_given_name_null_flavor: sender.person_given_name_null_flavor,
			person_middle_name: sender.person_middle_name,
			person_middle_name_null_flavor: sender.person_middle_name_null_flavor,
			person_family_name: sender.person_family_name,
			person_family_name_null_flavor: sender.person_family_name_null_flavor,
			telephone: sender.telephone,
			fax: sender.fax,
			email: sender.email,
		},
	)
	.await?;

	Ok(())
}

async fn sender_import_from_presave(
	ctx: &Ctx,
	mm: &ModelManager,
	sender: SenderPresave,
) -> Result<c_helpers::SenderImport> {
	let responsible_people =
		SenderPresaveResponsiblePersonBmc::list_by_parent(ctx, mm, sender.id)
			.await
			.map_err(Error::Model)?;
	let responsible = responsible_people
		.iter()
		.filter(|person| !person.deleted)
		.find(|person| person.is_default);

	let organization_name = sender
		.organization_name
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::InvalidImportRequest {
			message: "selected Sender has no organization name".to_string(),
		})?;
	let sender_type = sender
		.sender_type
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::InvalidImportRequest {
			message: "selected Sender has no sender type".to_string(),
		})?;

	Ok(c_helpers::SenderImport {
		sender_type: Some(sender_type),
		health_professional_type_kr1: None,
		organization_name: Some(organization_name),
		department: responsible
			.as_ref()
			.and_then(|person| person.department.clone()),
		street_address: sender.street_address,
		city: sender.city,
		state: sender.state,
		postcode: sender.postcode,
		country_code: sender.country_code,
		person_title: responsible
			.as_ref()
			.and_then(|person| person.person_title.clone()),
		person_title_null_flavor: None,
		person_given_name: responsible
			.as_ref()
			.and_then(|person| person.person_given_name.clone()),
		person_given_name_null_flavor: None,
		person_middle_name: responsible
			.as_ref()
			.and_then(|person| person.person_middle_name.clone()),
		person_middle_name_null_flavor: None,
		person_family_name: responsible
			.as_ref()
			.and_then(|person| person.person_family_name.clone()),
		person_family_name_null_flavor: None,
		telephone: sender.telephone,
		fax: sender.fax,
		email: sender.email,
	})
}

async fn import_c_3_primary_sources(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let primary_sources = c_helpers::parse_primary_sources(xml)?;
	if primary_sources.is_empty() {
		return Ok(());
	}

	for (idx, primary) in primary_sources.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let primary_id = if let Some((id,)) = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM primary_sources WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
		{
			id
		} else {
			PrimarySourceBmc::create(
				ctx,
				mm,
					PrimarySourceForCreate {
						case_id,
						source_reporter_presave_id: None,
						sequence_number: seq,
					reporter_title: primary.reporter_title.clone(),
					reporter_title_null_flavor: primary.reporter_title_null_flavor.clone(),
					reporter_given_name: primary.reporter_given_name.clone(),
					reporter_given_name_null_flavor: primary.reporter_given_name_null_flavor.clone(),
					reporter_middle_name: primary.reporter_middle_name.clone(),
					reporter_middle_name_null_flavor: primary.reporter_middle_name_null_flavor.clone(),
					reporter_family_name: primary.reporter_family_name.clone(),
					reporter_family_name_null_flavor: primary.reporter_family_name_null_flavor.clone(),
					organization: primary.organization.clone(),
					organization_null_flavor: primary.organization_null_flavor.clone(),
					department: primary.department.clone(),
					department_null_flavor: primary.department_null_flavor.clone(),
					street: primary.street.clone(),
					street_null_flavor: primary.street_null_flavor.clone(),
					city: primary.city.clone(),
					city_null_flavor: primary.city_null_flavor.clone(),
					state: primary.state.clone(),
					state_null_flavor: primary.state_null_flavor.clone(),
					postcode: primary.postcode.clone(),
					postcode_null_flavor: primary.postcode_null_flavor.clone(),
					telephone: primary.telephone.clone(),
					telephone_null_flavor: primary.telephone_null_flavor.clone(),
					country_code: primary.country_code.clone(),
					email: primary.email.clone(),
					email_null_flavor: primary.email_null_flavor.clone(),
					qualification: primary.qualification.clone(),
					qualification_null_flavor: primary.qualification_null_flavor.clone(),
					qualification_kr1: primary.qualification_kr1.clone(),
					primary_source_regulatory: primary.primary_source_regulatory.clone(),
				},
			)
			.await?
		};

		PrimarySourceBmc::update(
			ctx,
			mm,
			primary_id,
			PrimarySourceForUpdate {
				source_reporter_presave_id: None,
				reporter_title: primary.reporter_title,
				reporter_title_null_flavor: primary.reporter_title_null_flavor,
				reporter_given_name: primary.reporter_given_name,
				reporter_given_name_null_flavor: primary
					.reporter_given_name_null_flavor,
				reporter_middle_name: primary.reporter_middle_name,
				reporter_middle_name_null_flavor: primary
					.reporter_middle_name_null_flavor,
				reporter_family_name: primary.reporter_family_name,
				reporter_family_name_null_flavor: primary
					.reporter_family_name_null_flavor,
				organization: primary.organization,
				organization_null_flavor: primary.organization_null_flavor,
				department: primary.department,
				department_null_flavor: primary.department_null_flavor,
				street: primary.street,
				street_null_flavor: primary.street_null_flavor,
				city: primary.city,
				city_null_flavor: primary.city_null_flavor,
				state: primary.state,
				state_null_flavor: primary.state_null_flavor,
				postcode: primary.postcode,
				postcode_null_flavor: primary.postcode_null_flavor,
				telephone: primary.telephone,
				telephone_null_flavor: primary.telephone_null_flavor,
				country_code: primary.country_code,
				email: primary.email,
				email_null_flavor: primary.email_null_flavor,
				qualification: primary.qualification,
				qualification_null_flavor: primary.qualification_null_flavor,
				qualification_kr1: primary.qualification_kr1,
				primary_source_regulatory: primary.primary_source_regulatory,
			},
		)
		.await?;
	}

	Ok(())
}

async fn import_c_4_case_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let other_ids = c_helpers::parse_other_case_identifiers(xml)?;
	for (idx, entry) in other_ids.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM other_case_identifiers WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			OtherCaseIdentifierBmc::update(
				ctx,
				mm,
				id,
				OtherCaseIdentifierForUpdate {
					source_of_identifier: Some(entry.source_of_identifier),
					case_identifier: Some(entry.case_identifier),
				},
			)
			.await?;
		} else {
			let _ = OtherCaseIdentifierBmc::create(
				ctx,
				mm,
				OtherCaseIdentifierForCreate {
					case_id,
					sequence_number: seq,
					source_of_identifier: entry.source_of_identifier,
					case_identifier: entry.case_identifier,
				},
			)
			.await?;
		}
	}

	let linked = c_helpers::parse_linked_reports(xml)?;
	for (idx, entry) in linked.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM linked_report_numbers WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			LinkedReportNumberBmc::update(
				ctx,
				mm,
				id,
				LinkedReportNumberForUpdate {
					linked_report_number: Some(entry.linked_report_number),
				},
			)
			.await?;
		} else {
			let _ = LinkedReportNumberBmc::create(
				ctx,
				mm,
				LinkedReportNumberForCreate {
					case_id,
					sequence_number: seq,
					linked_report_number: entry.linked_report_number,
				},
			)
			.await?;
		}
	}

	Ok(())
}

async fn import_c_4_documents_held_by_sender(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let documents = c_helpers::parse_documents_held_by_sender(xml)?;
	for (idx, doc) in documents.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM documents_held_by_sender WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			DocumentsHeldBySenderBmc::update(
				ctx,
				mm,
				id,
				DocumentsHeldBySenderForUpdate {
					title: doc.title,
					document_base64: doc.document_base64,
					file_name: doc.file_name,
					media_type: doc.media_type,
					representation: doc.representation,
					compression: doc.compression,
					sequence_number: Some(seq),
				},
			)
			.await?;
		} else {
			let _ = DocumentsHeldBySenderBmc::create(
				ctx,
				mm,
				DocumentsHeldBySenderForCreate {
					case_id,
					title: doc.title,
					document_base64: doc.document_base64,
					file_name: doc.file_name,
					media_type: doc.media_type,
					representation: doc.representation,
					compression: doc.compression,
					sequence_number: seq,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn import_c_4_literature_references(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let references = c_helpers::parse_literature_references(xml)?;
	for (idx, entry) in references.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let reference_text = entry.reference_text.clone();
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM literature_references WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			LiteratureReferenceBmc::update(
				ctx,
				mm,
				id,
				LiteratureReferenceForUpdate {
					reference_text: reference_text.clone(),
					reference_text_null_flavor: entry.reference_text_null_flavor,
					sequence_number: Some(seq),
					document_base64: entry.document_base64,
					file_name: entry.file_name,
					media_type: entry.media_type,
					representation: entry.representation,
					compression: entry.compression,
				},
			)
			.await?;
		} else {
			let _ = LiteratureReferenceBmc::create(
				ctx,
				mm,
				LiteratureReferenceForCreate {
					case_id,
					reference_text,
					reference_text_null_flavor: entry.reference_text_null_flavor,
					sequence_number: seq,
					document_base64: entry.document_base64,
					file_name: entry.file_name,
					media_type: entry.media_type,
					representation: entry.representation,
					compression: entry.compression,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn import_c_5_study_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some(study) = c_helpers::parse_study_information(xml)? else {
		return Ok(());
	};

	let (study_id,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Uuid,)>(
				"INSERT INTO study_information (
						case_id,
						study_name,
						study_name_null_flavor,
						sponsor_study_number,
						sponsor_study_number_null_flavor,
						study_type_reaction,
						study_type_reaction_kr1,
						fda_ind_number_occurred,
						fda_pre_anda_number_occurred,
						created_at,
						updated_at,
						created_by
					) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,NOW(),NOW(),$10)
					ON CONFLICT (case_id) DO UPDATE SET
						study_name = EXCLUDED.study_name,
						study_name_null_flavor = EXCLUDED.study_name_null_flavor,
						sponsor_study_number = EXCLUDED.sponsor_study_number,
						sponsor_study_number_null_flavor = EXCLUDED.sponsor_study_number_null_flavor,
						study_type_reaction = EXCLUDED.study_type_reaction,
						study_type_reaction_kr1 = EXCLUDED.study_type_reaction_kr1,
						fda_ind_number_occurred = EXCLUDED.fda_ind_number_occurred,
						fda_pre_anda_number_occurred = EXCLUDED.fda_pre_anda_number_occurred,
						updated_at = NOW(),
						updated_by = $10
					RETURNING id",
			)
			.bind(case_id)
			.bind(study.study_name)
			.bind(study.study_name_null_flavor)
			.bind(study.sponsor_study_number)
			.bind(study.sponsor_study_number_null_flavor)
			.bind(study.study_type_reaction)
			.bind(study.study_type_reaction_kr1)
			.bind(study.fda_ind_number_occurred)
			.bind(study.fda_pre_anda_number_occurred)
			.bind(ctx.user_id()),
		)
		.await
		.map_err(model::Error::from)?;
	let (study_visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM study_information WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?;
	if study_visible_count != 1 {
		return Err(Error::Model(model::Error::Store(format!(
			"section C study write invariant failed for case {case_id}: visible_count={study_visible_count}"
		))));
	}

	mm.dbx()
		.execute(
			sqlx::query(
				"DELETE FROM study_registration_numbers WHERE study_information_id = $1",
			)
			.bind(study_id),
		)
		.await
		.map_err(model::Error::from)?;

	for (idx, reg) in study.registrations.into_iter().enumerate() {
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO study_registration_numbers (
							study_information_id,
							registration_number,
							registration_number_null_flavor,
							country_code,
							country_code_null_flavor,
							sequence_number,
							created_at,
							updated_at,
							created_by
						) VALUES ($1,$2,$3,$4,$5,$6,NOW(),NOW(),$7)",
				)
				.bind(study_id)
				.bind(reg.registration_number)
				.bind(reg.registration_number_null_flavor)
				.bind(reg.country_code)
				.bind(reg.country_code_null_flavor)
				.bind((idx + 1) as i32)
				.bind(ctx.user_id()),
			)
			.await
			.map_err(model::Error::from)?;
	}
	let (reg_visible_count,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*) FROM study_registration_numbers WHERE study_information_id = $1",
			)
			.bind(study_id),
		)
		.await
		.map_err(model::Error::from)?;
	if reg_visible_count < 0 {
		return Err(Error::Model(model::Error::Store(
			"section C study registration invariant failed".to_string(),
		)));
	}

	mm.dbx().execute(sqlx::query("DELETE FROM study_fda_cross_reported_inds WHERE study_information_id = $1").bind(study_id)).await.map_err(model::Error::from)?;
	for (idx, (ind_number, ind_number_null_flavor)) in
		study.cross_reported_inds.into_iter().enumerate()
	{
		mm.dbx().execute(sqlx::query("INSERT INTO study_fda_cross_reported_inds (study_information_id, ind_number, ind_number_null_flavor, sequence_number, created_at, updated_at, created_by) VALUES ($1,$2,$3,$4,NOW(),NOW(),$5)").bind(study_id).bind(ind_number).bind(ind_number_null_flavor).bind((idx + 1) as i32).bind(ctx.user_id())).await.map_err(model::Error::from)?;
	}
	Ok(())
}

async fn import_c_6_receiver_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some(receiver) = c_helpers::parse_receiver_information(xml)? else {
		return Ok(());
	};
	let receiver_type = receiver.receiver_type;
	let organization_name = receiver.organization_name;
	let department = receiver.department;
	let street_address = receiver.street_address;
	let city = receiver.city;
	let state_province = receiver.state_province;
	let postcode = receiver.postcode;
	let country_code = receiver.country_code;
	let telephone = receiver.telephone;
	let fax = receiver.fax;
	let email = receiver.email;

	if ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id)
		.await?
		.is_some()
	{
		ReceiverInformationBmc::update_by_case(
			ctx,
			mm,
			case_id,
			ReceiverInformationForUpdate {
				source_receiver_presave_id: None,
				receiver_type,
				organization_name,
				department,
				street_address,
				city,
				state_province,
				postcode,
				country_code,
				telephone,
				fax,
				email,
			},
		)
		.await?;
	} else {
		let _ = ReceiverInformationBmc::create(
			ctx,
			mm,
			ReceiverInformationForCreate {
				case_id,
				source_receiver_presave_id: None,
				receiver_type,
				organization_name,
				department,
				street_address,
				city,
				state_province,
				postcode,
				country_code,
				telephone,
				fax,
				email,
			},
		)
		.await?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use time::Month;

	fn date(year: i32, month: Month, day: u8) -> Date {
		Date::from_calendar_date(year, month, day).expect("valid test date")
	}

	fn report() -> CSafetyReportImport {
		CSafetyReportImport {
			transmission_date: Some("20240110000000".to_string()),
			report_type: Some("1".to_string()),
			date_first_received_from_source: Some("20240105".to_string()),
			date_of_most_recent_information: Some("20240108".to_string()),
			fulfil_expedited_criteria: Some(false),
			fulfil_expedited_criteria_null_flavor: None,
			additional_documents_available: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			combination_product_report_indicator_null_flavor: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			nullification_code: None,
			nullification_reason: None,
		}
	}

	#[test]
	fn import_date_settings_leave_date_order_to_business_validation() {
		let import_date = date(2024, Month::February, 1);
		let mut report = report();

		let result = apply_c_safety_report_import_settings(
			&mut report,
			&CImportSettings {
				update_date_of_creation: false,
				update_most_recent_info_date: false,
				update_report_first_received_date: true,
				apply_sender_info_to_imported_cases: false,
				selected_sender_presave_id: None,
				import_date: None,
			},
			import_date,
		);

		assert!(result.is_ok());
	}

	#[test]
	fn import_preserves_most_recent_date_later_than_creation_date() {
		let import_date = date(2024, Month::February, 1);
		let mut report = report();

		let result = apply_c_safety_report_import_settings(
			&mut report,
			&CImportSettings {
				update_date_of_creation: false,
				update_most_recent_info_date: true,
				update_report_first_received_date: false,
				apply_sender_info_to_imported_cases: false,
				selected_sender_presave_id: None,
				import_date: None,
			},
			import_date,
		);

		assert!(result.is_ok());
		assert_eq!(
			report.date_of_most_recent_information.as_deref(),
			Some("20240201")
		);
		assert_eq!(report.transmission_date.as_deref(), Some("20240110000000"));
	}
}
