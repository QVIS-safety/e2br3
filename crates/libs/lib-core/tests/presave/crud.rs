use super::*;

#[serial]
#[tokio::test]
async fn product_presave_round_trips_receiver_presave_id() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("Receiver-linked product sender".into()),
	)
	.await?;
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Receiver Link {}", Uuid::new_v4())),
			day_count_rule: None,
			nsae_solicited_day_count: None,
			nsae_solicited_not_applicable: None,
			nsae_non_solicited_day_count: None,
			nsae_non_solicited_not_applicable: None,
			sae_solicited_day_count: None,
			sae_solicited_not_applicable: None,
			sae_non_solicited_day_count: None,
			sae_non_solicited_not_applicable: None,
			description: None,
		},
	)
	.await?;
	let mut input = product_presave_create(
		RegulatoryAuthority::Fda,
		"Receiver-linked product".into(),
		sender_id,
	);
	input.receiver_presave_id = Some(receiver_id);
	let product_id = ProductPresaveBmc::create(&ctx, &mm, input).await?;

	assert_eq!(
		ProductPresaveBmc::get(&ctx, &mm, product_id)
			.await?
			.receiver_presave_id,
		Some(receiver_id)
	);
	expect_conflict_error(
		PresaveLifecycleService::archive(
			&ctx,
			&mm,
			PresaveKind::Receiver,
			receiver_id,
		)
		.await,
		"receiver presave is in use",
	);
	let deleted_receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Deleted Receiver {}", Uuid::new_v4())),
			day_count_rule: None,
			nsae_solicited_day_count: None,
			nsae_solicited_not_applicable: None,
			nsae_non_solicited_day_count: None,
			nsae_non_solicited_not_applicable: None,
			sae_solicited_day_count: None,
			sae_solicited_not_applicable: None,
			sae_non_solicited_day_count: None,
			sae_non_solicited_not_applicable: None,
			description: None,
		},
	)
	.await?;
	PresaveLifecycleService::archive(
		&ctx,
		&mm,
		PresaveKind::Receiver,
		deleted_receiver_id,
	)
	.await?;
	let mut deleted_input = product_presave_create(
		RegulatoryAuthority::Fda,
		"Deleted receiver product".into(),
		sender_id,
	);
	deleted_input.receiver_presave_id = Some(deleted_receiver_id);
	expect_conflict_error(
		ProductPresaveBmc::create(&ctx, &mm, deleted_input).await,
		"active receiver presave",
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn study_presave_round_trips_fda_regional_numbers() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("FDA study sender".into()),
	)
	.await?;
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Fda,
			"FDA study product".into(),
			sender_id,
		),
	)
	.await?;
	let mut input =
		study_presave_create_for_product("FDA regional study".into(), product_id);
	input.fda_ind_number_occurred = Some("123456".into());
	input.fda_pre_anda_number_occurred = Some("234567".into());
	let id = StudyPresaveBmc::create(&ctx, &mm, input).await?;
	let saved = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	assert_eq!(saved.fda_ind_number_occurred.as_deref(), Some("123456"));
	assert_eq!(
		saved.fda_pre_anda_number_occurred.as_deref(),
		Some("234567")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_crud_roundtrip() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: Some(true),
			sender_type: Some("1".into()),
			organization_name: Some(format!("Sender Org Before {suffix}")),
			organization_name_notation: None,
			street_address: Some("1 Sender St".into()),
			city: Some("Seoul".into()),
			state: Some("11".into()),
			postcode: Some("04524".into()),
			country_code: Some("KR".into()),
			telephone: Some("02-1111-2222".into()),
			fax: Some("02-1111-3333".into()),
			email: Some("sender@example.com".into()),
		},
	)
	.await?;
	let sender = SenderPresaveBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(
		sender.organization_name.as_deref(),
		Some(format!("Sender Org Before {suffix}").as_str())
	);

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Receiver Org {suffix}")),
			day_count_rule: Some("calendar".into()),
			nsae_solicited_day_count: Some(15),
			nsae_solicited_not_applicable: Some(false),
			nsae_non_solicited_day_count: Some(15),
			nsae_non_solicited_not_applicable: Some(false),
			sae_solicited_day_count: Some(7),
			sae_solicited_not_applicable: Some(false),
			sae_non_solicited_day_count: Some(7),
			sae_non_solicited_not_applicable: Some(false),
			description: Some("FDA routing".into()),
		},
	)
	.await?;

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			sender_presave_id: Some(sender_id),
			receiver_presave_id: None,
			product_id: Some(format!("PRODUCT-{suffix}")),
			medicinal_product: Some(format!("Medicinal Product {suffix}")),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: Some("Brand".into()),
			original_manufacturer: None,
			product_description: Some("Product description".into()),
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: Some("KR".into()),
			drug_authorization_number: Some("AUTH-1".into()),
			drug_authorization_country: Some("KR".into()),
			drug_authorization_holder: Some("Holder".into()),
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	let product = ProductPresaveBmc::get(&ctx, &mm, product_id).await?;
	assert_eq!(
		product.medicinal_product.as_deref(),
		Some(format!("Medicinal Product {suffix}").as_str())
	);

	let reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			reporter_title: Some("Dr".into()),
			reporter_given_name: Some("Casey".into()),
			reporter_middle_name: None,
			reporter_family_name: Some("Reporter".into()),
			organization: Some(format!("Reporter Org {suffix}")),
			department: Some("PV".into()),
			street: Some("2 Reporter St".into()),
			city: Some("Busan".into()),
			state: None,
			postcode: Some("48000".into()),
			telephone: Some("051-111-2222".into()),
			country_code: Some("KR".into()),
			qualification: Some("1".into()),
			qualification_kr1: None,
			qualification_null_flavor: None,
			primary_source_regulatory: Some("1".into()),
			..Default::default()
		},
	)
	.await?;
	let reporter = ReporterPresaveBmc::get(&ctx, &mm, reporter_id).await?;
	assert_eq!(reporter.reporter_family_name.as_deref(), Some("Reporter"));

	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			product_presave_id: Some(product_id),
			study_name: Some(format!("Study Name {suffix}")),
			study_name_null_flavor: None,
			study_name_notation: Some("Study Name Notation".into()),
			sponsor_study_number: Some(format!("ST-001-{suffix}")),
			sponsor_study_number_null_flavor: None,
			sponsor_study_number_kind: Some("PROTOCOL_NO".into()),
			study_type_reaction: Some("1".into()),
			study_type_reaction_kr1: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			edc_sync: Some(true),
			exclude_case_key_from_sync: Some(true),
		},
	)
	.await?;
	let study = StudyPresaveBmc::get(&ctx, &mm, study_id).await?;
	assert_eq!(
		study.sponsor_study_number.as_deref(),
		Some(format!("ST-001-{suffix}").as_str())
	);
	assert_eq!(
		study.study_name_notation.as_deref(),
		Some("Study Name Notation")
	);
	assert_eq!(
		study.sponsor_study_number_kind.as_deref(),
		Some("PROTOCOL_NO")
	);
	assert_eq!(study.exclude_case_key_from_sync, Some(true));

	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			template_title: Some("Case narrative template".into()),
			case_narrative: Some("Case narrative text".into()),
			case_narrative_notation: Some("Case narrative notation".into()),
			additional_information: Some("Sponsor additional information".into()),
		},
	)
	.await?;
	let narrative = NarrativePresaveBmc::get(&ctx, &mm, narrative_id).await?;
	assert_eq!(
		narrative.template_title.as_deref(),
		Some("Case narrative template")
	);
	assert_eq!(
		narrative.case_narrative.as_deref(),
		Some("Case narrative text")
	);
	assert_eq!(
		narrative.case_narrative_notation.as_deref(),
		Some("Case narrative notation")
	);
	assert_eq!(
		narrative.additional_information.as_deref(),
		Some("Sponsor additional information")
	);

	SenderPresaveBmc::update(
		&ctx,
		&mm,
		sender_id,
		SenderPresaveForUpdate {
			organization_name: Some(format!("Sender Org After {suffix}")),
			..Default::default()
		},
	)
	.await?;
	let updated_sender = SenderPresaveBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(
		updated_sender.organization_name.as_deref(),
		Some(format!("Sender Org After {suffix}").as_str())
	);
	let ich_senders = SenderPresaveBmc::list(&ctx, &mm, None).await?;
	assert!(
		ich_senders.iter().any(|sender| sender.id == sender_id
			&& sender.organization_name.as_deref()
				== Some(format!("Sender Org After {suffix}").as_str())),
		"updated sender should appear in presave list results"
	);

	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ReporterPresaveBmc::delete(&ctx, &mm, reporter_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}
