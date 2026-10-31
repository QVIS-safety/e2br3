use super::*;

#[test]
fn product_update_distinguishes_missing_null_and_present_optional_values() {
	let missing = serde_json::from_value::<ProductPresaveForUpdate>(json!({}))
		.expect("missing patch fields deserialize");
	assert_eq!(missing.receiver_presave_id, None);
	assert_eq!(missing.investigational_product_blinded, None);

	let cleared = serde_json::from_value::<ProductPresaveForUpdate>(json!({
		"receiverPresaveId": null,
		"investigationalProductBlinded": null
	}))
	.expect("null patch fields deserialize");
	assert_eq!(cleared.receiver_presave_id, Some(None));
	assert_eq!(cleared.investigational_product_blinded, Some(None));

	let receiver_id = Uuid::new_v4();
	let present = serde_json::from_value::<ProductPresaveForUpdate>(json!({
		"receiverPresaveId": receiver_id,
		"investigationalProductBlinded": true
	}))
	.expect("present patch fields deserialize");
	assert_eq!(present.receiver_presave_id, Some(Some(receiver_id)));
	assert_eq!(present.investigational_product_blinded, Some(Some(true)));
}

#[serial]
#[tokio::test]
async fn authorityless_union_fields_are_allowed() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Authorityless Sender Org {suffix}")),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;

	let product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authorityless Union Product {suffix}"),
		sender_id,
	);
	let product_id = ProductPresaveBmc::create(&ctx, &mm, product).await?;
	ProductPresaveBmc::update(
		&ctx,
		&mm,
		product_id,
		ProductPresaveForUpdate {
			medicinal_product: Some("Authorityless Union Product Updated".into()),
			..Default::default()
		},
	)
	.await?;

	let mut reporter = reporter_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authorityless Reporter {suffix}"),
	);
	reporter.primary_source_regulatory = Some("1".into());
	let reporter_id = ReporterPresaveBmc::create(&ctx, &mm, reporter).await?;
	ReporterPresaveBmc::update(
		&ctx,
		&mm,
		reporter_id,
		ReporterPresaveForUpdate {
			primary_source_regulatory: Some("1".into()),
			..Default::default()
		},
	)
	.await?;

	let mut study = study_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authorityless Study {suffix}"),
	);
	study.product_presave_id = Some(product_id);
	study.exclude_case_key_from_sync = Some(true);
	study.sponsor_study_number_kind = Some("STUDY_NO".into());
	let study_id = StudyPresaveBmc::create(&ctx, &mm, study).await?;
	assert_eq!(
		StudyPresaveBmc::get(&ctx, &mm, study_id)
			.await?
			.sponsor_study_number_kind
			.as_deref(),
		Some("STUDY_NO")
	);
	StudyPresaveProductBmc::create(
		&ctx,
		&mm,
		StudyPresaveProductForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			product_presave_id: Some(product_id),
			product_name: Some("Study Product Child".into()),
			deleted: Some(false),
		},
	)
	.await?;

	let mut invalid_kind_study = study_presave_create(
		RegulatoryAuthority::Ich,
		format!("Authorityless Study Invalid Kind {suffix}"),
	);
	invalid_kind_study.product_presave_id = Some(product_id);
	invalid_kind_study.sponsor_study_number_kind = Some("other_no".into());
	expect_validation_error(
		StudyPresaveBmc::create(&ctx, &mm, invalid_kind_study).await,
		"sponsor_study_number_kind",
	);
	StudyPresaveBmc::update(
		&ctx,
		&mm,
		study_id,
		StudyPresaveForUpdate {
			sponsor_study_number_kind: Some(String::new()),
			..Default::default()
		},
	)
	.await?;
	assert_eq!(
		StudyPresaveBmc::get(&ctx, &mm, study_id)
			.await?
			.sponsor_study_number_kind
			.as_deref(),
		None
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn reporter_presave_accepts_field_specific_null_flavors() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			reporter_title: None,
			reporter_title_null_flavor: Some("UNK".into()),
			reporter_given_name: None,
			reporter_given_name_null_flavor: Some("ASKU".into()),
			reporter_middle_name: None,
			reporter_middle_name_null_flavor: None,
			reporter_family_name: None,
			reporter_family_name_null_flavor: None,
			organization: None,
			organization_null_flavor: Some("NASK".into()),
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: None,
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			telephone: None,
			telephone_null_flavor: Some("MSK".into()),
			reporter_email: None,
			reporter_email_null_flavor: None,
			country_code: None,
			qualification: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			qualification_null_flavor: Some("UNK".into()),
		},
	)
	.await?;

	let saved = ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	assert_eq!(saved.reporter_title_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(
		saved.reporter_given_name_null_flavor.as_deref(),
		Some("ASKU")
	);
	assert_eq!(saved.organization_null_flavor.as_deref(), Some("NASK"));
	assert_eq!(saved.telephone_null_flavor.as_deref(), Some("MSK"));
	assert_eq!(saved.qualification_null_flavor.as_deref(), Some("UNK"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn reporter_presave_rejects_invalid_field_specific_null_flavors() -> Result<()>
{
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	expect_validation_error(
		ReporterPresaveBmc::create(
			&ctx,
			&mm,
			ReporterPresaveForCreate {
				reporter_title: None,
				reporter_title_null_flavor: None,
				reporter_given_name: Some(format!("Reporter {suffix}")),
				reporter_given_name_null_flavor: Some("UNK".into()),
				reporter_middle_name: None,
				reporter_middle_name_null_flavor: None,
				reporter_family_name: None,
				reporter_family_name_null_flavor: None,
				organization: Some(format!("Reporter Org {suffix}")),
				organization_null_flavor: Some("MSK".into()),
				department: None,
				department_null_flavor: None,
				street: None,
				street_null_flavor: None,
				city: None,
				city_null_flavor: None,
				state: None,
				state_null_flavor: None,
				postcode: None,
				postcode_null_flavor: None,
				telephone: None,
				telephone_null_flavor: None,
				reporter_email: None,
				reporter_email_null_flavor: None,
				country_code: None,
				qualification: Some("1".into()),
				qualification_kr1: None,
				primary_source_regulatory: None,
				qualification_null_flavor: Some("MSK".into()),
			},
		)
		.await,
		"reporter_given_name_null_flavor",
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn study_presave_registration_numbers_enforce_registration_number_max_length(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!(
				"Registration Length Sender Org {suffix}"
			)),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Ich,
			format!("Registration Length Product {suffix}"),
			sender_id,
		),
	)
	.await?;
	let mut study = study_presave_create(
		RegulatoryAuthority::Ich,
		format!("Registration Length Study {suffix}"),
	);
	study.product_presave_id = Some(product_id);
	let study_id = StudyPresaveBmc::create(&ctx, &mm, study).await?;

	let registration_id = StudyPresaveRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyPresaveRegistrationNumberForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			registration_number: Some("REG-before".into()),
			registration_number_null_flavor: None,
			country_code: Some("KR".into()),
			country_code_null_flavor: None,
			deleted: Some(false),
		},
	)
	.await?;
	expect_validation_error(
		StudyPresaveRegistrationNumberBmc::create(
			&ctx,
			&mm,
			StudyPresaveRegistrationNumberForCreate {
				study_presave_id: study_id,
				sequence_number: 2,
				registration_number: Some("R".repeat(51)),
				registration_number_null_flavor: None,
				country_code: Some("KR".into()),
				country_code_null_flavor: None,
				deleted: Some(false),
			},
		)
		.await,
		"registration_number` must be at most 50 characters",
	);
	expect_validation_error(
		StudyPresaveRegistrationNumberBmc::update(
			&ctx,
			&mm,
			registration_id,
			StudyPresaveRegistrationNumberForUpdate {
				registration_number: Some("R".repeat(51)),
				..Default::default()
			},
		)
		.await,
		"registration_number` must be at most 50 characters",
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_enforce_minimal_identity_requirements(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let valid_sender_without_parent_person = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Valid Sender Org {suffix}")),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	assert!(
		!valid_sender_without_parent_person.is_nil(),
		"sender parent identity should not require person_given_name",
	);
	expect_validation_error(
		ReceiverPresaveBmc::create(
			&ctx,
			&mm,
			ReceiverPresaveForCreate {
				receiver_type: None,
				organization_name: Some("Invalid Receiver Org".into()),
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
		.await,
		"receiver_type and organization_name",
	);
	expect_validation_error(
		ProductPresaveBmc::create(
			&ctx,
			&mm,
			ProductPresaveForCreate {
				sender_presave_id: None,
				receiver_presave_id: None,
				product_id: None,
				medicinal_product: None,
				medicinal_product_notation: None,
				preapproval_ip_name: None,
				brand_name: None,
				original_manufacturer: None,
				product_description: None,
				mpid: None,
				mpid_version: None,
				mfds_mpid: None,
				mfds_mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				drug_authorization_number: None,
				drug_authorization_country: None,
				drug_authorization_holder: None,
				holder_applicant_name_notation: None,
			},
		)
		.await,
		"sender_presave_id and product_id or preapproval_ip_name",
	);
	for (_label, reporter_given_name, organization, qualification) in [
		("given name", None, Some("Invalid Reporter Org"), Some("1")),
		("organization", Some("Invalid"), None, Some("1")),
		(
			"qualification",
			Some("Invalid"),
			Some("Invalid Reporter Org"),
			None,
		),
	] {
		expect_validation_error(
			ReporterPresaveBmc::create(
				&ctx,
				&mm,
				ReporterPresaveForCreate {
					reporter_title: None,
					reporter_given_name: reporter_given_name.map(str::to_string),
					reporter_middle_name: None,
					reporter_family_name: None,
					organization: organization.map(str::to_string),
					department: None,
					street: None,
					city: None,
					state: None,
					postcode: None,
					telephone: None,
					country_code: None,
					qualification: qualification.map(str::to_string),
					qualification_kr1: None,
					qualification_null_flavor: None,
					primary_source_regulatory: None,
					..Default::default()
				},
			)
			.await,
			"reporter_given_name, organization, and qualification",
		);
	}
	expect_validation_error(
		StudyPresaveBmc::create(
			&ctx,
			&mm,
			StudyPresaveForCreate {
				product_presave_id: None,
				study_name: Some("Invalid Study".into()),
				study_name_null_flavor: None,
				study_name_notation: None,
				sponsor_study_number: Some("INVALID-STUDY".into()),
				sponsor_study_number_null_flavor: None,
				sponsor_study_number_kind: None,
			study_type_reaction: None,
			study_type_reaction_kr1: None,
				fda_ind_number_occurred: None,
				fda_pre_anda_number_occurred: None,
				edc_sync: None,
				exclude_case_key_from_sync: None,
			},
		)
		.await,
		"product_presave_id, sponsor_study_number, study_name, and study_type_reaction",
	);
	let mut long_sponsor_number = study_presave_create(
		RegulatoryAuthority::Ich,
		format!("Invalid Study Presave Sponsor Length {suffix}"),
	);
	let sponsor_length_product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Ich,
			format!("Invalid Study Presave Sponsor Length Product {suffix}"),
			valid_sender_without_parent_person,
		),
	)
	.await?;
	long_sponsor_number.product_presave_id = Some(sponsor_length_product_id);
	long_sponsor_number.sponsor_study_number = Some("S".repeat(51));
	expect_validation_error(
		StudyPresaveBmc::create(&ctx, &mm, long_sponsor_number).await,
		"sponsor_study_number` must be at most 50 characters",
	);
	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			template_title: None,
			case_narrative: Some(format!("Minimal narrative {suffix}")),
			case_narrative_notation: None,
			additional_information: None,
		},
	)
	.await?;
	let narrative = NarrativePresaveBmc::get(&ctx, &mm, narrative_id).await?;
	assert_eq!(
		narrative.case_narrative.as_deref(),
		Some(format!("Minimal narrative {suffix}").as_str())
	);
	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_reject_duplicate_identity_within_org(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Duplicate Sender Org {suffix}")),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	expect_conflict_error(
		SenderPresaveBmc::create(
			&ctx,
			&mm,
			SenderPresaveForCreate {
				is_default: None,
				sender_type: Some("1".into()),
				organization_name: Some(format!(" duplicate sender org {suffix} ")),
				organization_name_notation: None,
				street_address: None,
				city: None,
				state: None,
				postcode: None,
				country_code: None,
				telephone: None,
				fax: None,
				email: None,
			},
		)
		.await,
		"sender presave duplicate",
	);

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Duplicate Receiver Org {suffix}")),
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
	expect_conflict_error(
		ReceiverPresaveBmc::create(
			&ctx,
			&mm,
			ReceiverPresaveForCreate {
				receiver_type: Some("Original Manufacturer".into()),
				organization_name: Some(format!(
					" duplicate receiver org {suffix} "
				)),
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
		.await,
		"receiver presave duplicate",
	);

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			sender_presave_id: Some(sender_id),
			receiver_presave_id: None,
			product_id: Some(format!("DUP-PRODUCT-{suffix}")),
			medicinal_product: None,
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	expect_conflict_error(
		ProductPresaveBmc::create(
			&ctx,
			&mm,
			ProductPresaveForCreate {
				sender_presave_id: Some(sender_id),
				receiver_presave_id: None,
				product_id: Some(format!(" dup-product-{suffix} ")),
				medicinal_product: None,
				medicinal_product_notation: None,
				preapproval_ip_name: None,
				brand_name: None,
				original_manufacturer: None,
				product_description: None,
				mpid: None,
				mpid_version: None,
				mfds_mpid: None,
				mfds_mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				drug_authorization_number: None,
				drug_authorization_country: None,
				drug_authorization_holder: None,
				holder_applicant_name_notation: None,
			},
		)
		.await,
		"product presave duplicate",
	);

	let reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			reporter_title: None,
			reporter_given_name: Some("Robin".into()),
			reporter_middle_name: None,
			reporter_family_name: None,
			organization: Some(format!("Duplicate Reporter Org {suffix}")),
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			telephone: None,
			country_code: None,
			qualification: Some("1".into()),
			qualification_kr1: None,
			qualification_null_flavor: None,
			primary_source_regulatory: None,
			..Default::default()
		},
	)
	.await?;
	expect_conflict_error(
		ReporterPresaveBmc::create(
			&ctx,
			&mm,
			ReporterPresaveForCreate {
				reporter_title: None,
				reporter_given_name: Some(" robin ".into()),
				reporter_middle_name: None,
				reporter_family_name: None,
				organization: Some(format!(" duplicate reporter org {suffix} ")),
				department: None,
				street: None,
				city: None,
				state: None,
				postcode: None,
				telephone: None,
				country_code: None,
				qualification: Some("1".into()),
				qualification_kr1: None,
				qualification_null_flavor: None,
				primary_source_regulatory: None,
				..Default::default()
			},
		)
		.await,
		"reporter presave duplicate",
	);

	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			product_presave_id: Some(product_id),
			study_name: Some("Duplicate Study".into()),
			study_name_null_flavor: None,
			study_name_notation: None,
			sponsor_study_number: Some(format!("DUP-STUDY-{suffix}")),
			sponsor_study_number_null_flavor: None,
			sponsor_study_number_kind: None,
			study_type_reaction: Some("1".into()),
			study_type_reaction_kr1: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			edc_sync: None,
			exclude_case_key_from_sync: None,
		},
	)
	.await?;
	expect_conflict_error(
		StudyPresaveBmc::create(
			&ctx,
			&mm,
			StudyPresaveForCreate {
				product_presave_id: Some(product_id),
				study_name: Some("Different Study".into()),
				study_name_null_flavor: None,
				study_name_notation: None,
				sponsor_study_number: Some(format!(" dup-study-{suffix} ")),
				sponsor_study_number_null_flavor: None,
				sponsor_study_number_kind: None,
				study_type_reaction: Some("2".into()),
				study_type_reaction_kr1: None,
				fda_ind_number_occurred: None,
				fda_pre_anda_number_occurred: None,
				edc_sync: None,
				exclude_case_key_from_sync: None,
			},
		)
		.await,
		"study presave duplicate",
	);

	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			template_title: None,
			case_narrative: Some(format!("Duplicate narrative body {suffix}")),
			case_narrative_notation: None,
			additional_information: None,
		},
	)
	.await?;
	expect_conflict_error(
		NarrativePresaveBmc::create(
			&ctx,
			&mm,
			NarrativePresaveForCreate {
				template_title: None,
				case_narrative: Some(format!(" duplicate narrative body {suffix} ")),
				case_narrative_notation: None,
				additional_information: None,
			},
		)
		.await,
		"narrative presave duplicate",
	);

	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ReporterPresaveBmc::delete(&ctx, &mm, reporter_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_receiver_allows_legacy_type_update() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	for legacy_type in ["1", "2", "3", "4", "5", "6"] {
		let receiver_id = Uuid::new_v4();
		let mut tx = mm.dbx().db().begin().await?;
		set_user_context(&mut tx, system_user_id()).await?;
		set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

		sqlx::query(
			"INSERT INTO receiver_presaves (
				id, organization_id, receiver_type, organization_name,
				created_by, updated_by
			)
			VALUES ($1, $2, $3, $4, $5, $5)",
		)
		.bind(receiver_id)
		.bind(demo_org_id())
		.bind(legacy_type)
		.bind(format!("Legacy Receiver Org {legacy_type} {suffix}"))
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await?;
		tx.commit().await?;

		ReceiverPresaveBmc::update(
			&ctx,
			&mm,
			receiver_id,
			ReceiverPresaveForUpdate {
				description: Some("legacy receiver still editable".into()),
				..Default::default()
			},
		)
		.await?;
		let receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
		assert_eq!(receiver.receiver_type.as_deref(), Some(legacy_type));
		assert_eq!(
			receiver.description.as_deref(),
			Some("legacy receiver still editable")
		);

		ReceiverPresaveBmc::update(
			&ctx,
			&mm,
			receiver_id,
			ReceiverPresaveForUpdate {
				receiver_type: Some(legacy_type.into()),
				description: Some("legacy receiver round-tripped".into()),
				..Default::default()
			},
		)
		.await?;
		let receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
		assert_eq!(receiver.receiver_type.as_deref(), Some(legacy_type));
		assert_eq!(
			receiver.description.as_deref(),
			Some("legacy receiver round-tripped")
		);

		ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	}

	Ok(())
}
