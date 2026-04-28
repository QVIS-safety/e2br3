use crate::ctx::Ctx;
use crate::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberForCreate, LinkedReportNumberForUpdate,
	OtherCaseIdentifierBmc, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use crate::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use crate::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderForCreate,
	DocumentsHeldBySenderForUpdate, LiteratureReferenceBmc,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySourceBmc,
	PrimarySourceForCreate, PrimarySourceForUpdate, SenderInformationBmc,
	SenderInformationForCreate, SenderInformationForUpdate,
};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{self, ModelManager};
use crate::xml::import_runtime::{helpers::c as c_helpers, shared};
use crate::xml::{error::Error, Result};
use sqlx::types::Uuid;

pub(crate) async fn import_section_c(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&shared::MessageHeaderExtract>,
) -> Result<()> {
	import_c_1_safety_report(ctx, mm, xml, case_id, header).await?;
	import_c_2_sender_information(ctx, mm, xml, case_id, header).await?;
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
	header: Option<&shared::MessageHeaderExtract>,
) -> Result<()> {
	let Some(report) =
		crate::xml::import_sections::c_safety_report::parse_c_safety_report(xml)?
	else {
		return Ok(());
	};

	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(err));
	}

	let receiver_organization = header.and_then(|h| h.message_receiver.clone());

	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO safety_report_identification (
					case_id,
					transmission_date,
					transmission_date_null_flavor,
					report_type,
					date_first_received_from_source,
					date_first_received_from_source_null_flavor,
					date_of_most_recent_information,
					date_of_most_recent_information_null_flavor,
					fulfil_expedited_criteria,
					local_criteria_report_type,
					combination_product_report_indicator,
					worldwide_unique_id,
					first_sender_type,
					additional_documents_available,
					nullification_code,
					nullification_reason,
					receiver_organization,
					created_at,
					updated_at,
					created_by
				) VALUES (
					$1,$2,NULL,$3,$4,NULL,$5,NULL,$6,$7,$8,$9,$10,$11,$12,$13,$14,NOW(),NOW(),$15
				)
				ON CONFLICT (case_id) DO UPDATE SET
					transmission_date = EXCLUDED.transmission_date,
					transmission_date_null_flavor = NULL,
					report_type = EXCLUDED.report_type,
					date_first_received_from_source = EXCLUDED.date_first_received_from_source,
					date_first_received_from_source_null_flavor = NULL,
					date_of_most_recent_information = EXCLUDED.date_of_most_recent_information,
					date_of_most_recent_information_null_flavor = NULL,
					fulfil_expedited_criteria = EXCLUDED.fulfil_expedited_criteria,
					local_criteria_report_type = EXCLUDED.local_criteria_report_type,
					combination_product_report_indicator = EXCLUDED.combination_product_report_indicator,
					worldwide_unique_id = EXCLUDED.worldwide_unique_id,
					first_sender_type = EXCLUDED.first_sender_type,
					additional_documents_available = EXCLUDED.additional_documents_available,
					nullification_code = EXCLUDED.nullification_code,
					nullification_reason = EXCLUDED.nullification_reason,
					receiver_organization = EXCLUDED.receiver_organization,
					updated_at = NOW(),
					updated_by = $15",
			)
			.bind(case_id)
			.bind(report.transmission_date)
			.bind(report.report_type)
			.bind(report.date_first_received_from_source)
			.bind(report.date_of_most_recent_information)
			.bind(report.fulfil_expedited_criteria)
			.bind(report.local_criteria_report_type)
			.bind(report.combination_product_report_indicator)
			.bind(report.worldwide_unique_id)
			.bind(report.first_sender_type)
			.bind(report.additional_documents_available)
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
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(model::Error::Store(format!(
			"section C safety report write invariant failed for case {case_id}: visible_count={visible_count}"
		))));
	}
	mm.dbx().commit_txn().await.map_err(model::Error::from)?;

	Ok(())
}

async fn import_c_2_sender_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&shared::MessageHeaderExtract>,
) -> Result<()> {
	let Some(sender) = c_helpers::parse_sender_information(xml, header)? else {
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
				sender_type: Some(sender.sender_type.clone()),
				organization_name: Some(sender.organization_name.clone()),
				department: sender.department.clone(),
				street_address: sender.street_address.clone(),
				city: sender.city.clone(),
				state: sender.state.clone(),
				postcode: sender.postcode.clone(),
				country_code: sender.country_code.clone(),
				person_title: sender.person_title.clone(),
				person_given_name: sender.person_given_name.clone(),
				person_middle_name: sender.person_middle_name.clone(),
				person_family_name: sender.person_family_name.clone(),
				telephone: sender.telephone.clone(),
				fax: sender.fax.clone(),
				email: sender.email.clone(),
			},
		)
		.await?
	};

	let _ = SenderInformationBmc::update(
		ctx,
		mm,
		sender_id,
		SenderInformationForUpdate {
			sender_type: Some(sender.sender_type),
			organization_name: Some(sender.organization_name),
			department: sender.department,
			street_address: sender.street_address,
			city: sender.city,
			state: sender.state,
			postcode: sender.postcode,
			country_code: sender.country_code,
			person_title: sender.person_title,
			person_given_name: sender.person_given_name,
			person_middle_name: sender.person_middle_name,
			person_family_name: sender.person_family_name,
			telephone: sender.telephone,
			fax: sender.fax,
			email: sender.email,
		},
	)
	.await;

	Ok(())
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
					sequence_number: seq,
					reporter_title: primary.reporter_title.clone(),
					reporter_given_name: primary.reporter_given_name.clone(),
					reporter_middle_name: primary.reporter_middle_name.clone(),
					reporter_family_name: primary.reporter_family_name.clone(),
					organization: primary.organization.clone(),
					department: primary.department.clone(),
					street: primary.street.clone(),
					city: primary.city.clone(),
					state: primary.state.clone(),
					postcode: primary.postcode.clone(),
					telephone: primary.telephone.clone(),
					country_code: primary.country_code.clone(),
					email: primary.email.clone(),
					qualification: primary.qualification.clone(),
					qualification_kr1: None,
					primary_source_regulatory: primary.primary_source_regulatory.clone(),
				},
			)
			.await?
		};

		let _ = PrimarySourceBmc::update(
			ctx,
			mm,
			primary_id,
			PrimarySourceForUpdate {
				reporter_title: primary.reporter_title,
				reporter_given_name: primary.reporter_given_name,
				reporter_middle_name: primary.reporter_middle_name,
				reporter_family_name: primary.reporter_family_name,
				organization: primary.organization,
				department: primary.department,
				street: primary.street,
				city: primary.city,
				state: primary.state,
				postcode: primary.postcode,
				telephone: primary.telephone,
				country_code: primary.country_code,
				email: primary.email,
				qualification: primary.qualification,
				qualification_kr1: None,
				primary_source_regulatory: primary.primary_source_regulatory,
			},
		)
		.await;
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
			let _ = OtherCaseIdentifierBmc::update(
				ctx,
				mm,
				id,
				OtherCaseIdentifierForUpdate {
					source_of_identifier: Some(entry.source_of_identifier),
					case_identifier: Some(entry.case_identifier),
				},
			)
			.await;
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
			let _ = LinkedReportNumberBmc::update(
				ctx,
				mm,
				id,
				LinkedReportNumberForUpdate {
					linked_report_number: Some(entry.linked_report_number),
				},
			)
			.await;
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
			let _ = DocumentsHeldBySenderBmc::update(
				ctx,
				mm,
				id,
				DocumentsHeldBySenderForUpdate {
					title: doc.title,
					document_base64: doc.document_base64,
					media_type: doc.media_type,
					representation: doc.representation,
					compression: doc.compression,
					sequence_number: Some(seq),
				},
			)
			.await;
		} else {
			let _ = DocumentsHeldBySenderBmc::create(
				ctx,
				mm,
				DocumentsHeldBySenderForCreate {
					case_id,
					title: doc.title,
					document_base64: doc.document_base64,
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
			let _ = LiteratureReferenceBmc::update(
				ctx,
				mm,
				id,
				LiteratureReferenceForUpdate {
					reference_text: Some(entry.reference_text),
					sequence_number: Some(seq),
					document_base64: entry.document_base64,
					media_type: entry.media_type,
					representation: entry.representation,
					compression: entry.compression,
				},
			)
			.await;
		} else {
			let _ = LiteratureReferenceBmc::create(
				ctx,
				mm,
				LiteratureReferenceForCreate {
					case_id,
					reference_text: entry.reference_text,
					sequence_number: seq,
					document_base64: entry.document_base64,
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

	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(err));
	}

	let (study_id,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Uuid,)>(
				"INSERT INTO study_information (
					case_id,
					study_name,
					sponsor_study_number,
					study_type_reaction,
					study_type_reaction_kr1,
					created_at,
					updated_at,
					created_by
				) VALUES ($1,$2,$3,$4,NULL,NOW(),NOW(),$5)
				ON CONFLICT (case_id) DO UPDATE SET
					study_name = EXCLUDED.study_name,
					sponsor_study_number = EXCLUDED.sponsor_study_number,
					study_type_reaction = EXCLUDED.study_type_reaction,
					study_type_reaction_kr1 = EXCLUDED.study_type_reaction_kr1,
					updated_at = NOW(),
					updated_by = $5
				RETURNING id",
			)
			.bind(case_id)
			.bind(study.study_name)
			.bind(study.sponsor_study_number)
			.bind(study.study_type_reaction)
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
		let _ = mm.dbx().rollback_txn().await;
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
						country_code,
						sequence_number,
						created_at,
						updated_at,
						created_by
					) VALUES ($1,$2,$3,$4,NOW(),NOW(),$5)",
				)
				.bind(study_id)
				.bind(reg.registration_number)
				.bind(reg.country_code)
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
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(model::Error::Store(
			"section C study registration invariant failed".to_string(),
		)));
	}
	mm.dbx().commit_txn().await.map_err(model::Error::from)?;

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
		let _ = ReceiverInformationBmc::update_by_case(
			ctx,
			mm,
			case_id,
			ReceiverInformationForUpdate {
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
		.await;
	} else {
		let _ = ReceiverInformationBmc::create(
			ctx,
			mm,
			ReceiverInformationForCreate {
				case_id,
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
