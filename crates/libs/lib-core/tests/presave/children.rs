use super::*;

#[serial]
#[tokio::test]
#[ignore = "requires full section presave child schema and audit tables"]
async fn section_presave_child_bmcs_crud_roundtrip() -> Result<()> {
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
			organization_name: Some(format!("Child Sender Org {suffix}")),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: Some("KR".into()),
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Child Receiver Org {suffix}")),
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
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			sender_presave_id: Some(sender_id),
			receiver_presave_id: None,
			product_id: Some(format!("CHILD-PRODUCT-{suffix}")),
			medicinal_product: Some("Child Product".into()),
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
	let fda_product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			sender_presave_id: Some(sender_id),
			receiver_presave_id: None,
			product_id: Some(format!("CHILD-FDA-PRODUCT-{suffix}")),
			medicinal_product: Some("Child FDA Product".into()),
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
	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			product_presave_id: Some(product_id),
			study_name: Some("Child Study".into()),
			study_name_null_flavor: None,
			study_name_notation: None,
			sponsor_study_number: Some(format!("CHILD-STUDY-{suffix}")),
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
	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			template_title: None,
			case_narrative: Some(format!("Child narrative {suffix}")),
			case_narrative_notation: None,
			additional_information: None,
		},
	)
	.await?;

	let gateway_late_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 20,
			gateway_authority: "fda".into(),
			sender_identifier: Some("gateway-late".into()),
			routing_identifier: None,
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(false),
			deleted: None,
		},
	)
	.await?;
	let gateway_first_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 10,
			gateway_authority: "mfds".into(),
			sender_identifier: Some("gateway-first".into()),
			routing_identifier: Some("route-before".into()),
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(true),
			deleted: None,
		},
	)
	.await?;
	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		gateway_first_id,
		SenderPresaveGatewayForUpdate {
			routing_identifier: Some("route-after".into()),
			..Default::default()
		},
	)
	.await?;
	let gateway_first =
		SenderPresaveGatewayBmc::get(&ctx, &mm, gateway_first_id).await?;
	assert_eq!(gateway_first.sender_presave_id, sender_id);
	assert_eq!(gateway_first.sequence_number, 10);
	assert_eq!(
		gateway_first.routing_identifier.as_deref(),
		Some("route-after")
	);
	assert_audit_changed_field(
		&mm,
		"sender_presave_gateways",
		gateway_first_id,
		"routing_identifier",
		json!("route-before"),
		json!("route-after"),
	)
	.await?;
	let gateways =
		SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, sender_id).await?;
	assert_eq!(gateways[0].id, gateway_first_id);
	assert_eq!(gateways[1].id, gateway_late_id);
	SenderPresaveGatewayBmc::delete(&ctx, &mm, gateway_late_id).await?;
	assert!(
		SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, sender_id)
			.await?
			.iter()
			.all(|gateway| gateway.id != gateway_late_id)
	);

	let responsible_id = SenderPresaveResponsiblePersonBmc::create(
		&ctx,
		&mm,
		SenderPresaveResponsiblePersonForCreate {
			sender_presave_id: sender_id,
			sequence_number: 1,
			department: Some("PV".into()),
			person_title: Some("Dr".into()),
			person_given_name: Some("Before".into()),
			person_middle_name: None,
			person_family_name: Some("Person".into()),
			is_default: Some(false),
			deleted: None,
		},
	)
	.await?;
	SenderPresaveResponsiblePersonBmc::update(
		&ctx,
		&mm,
		responsible_id,
		SenderPresaveResponsiblePersonForUpdate {
			person_given_name: Some("After".into()),
			is_default: Some(true),
			..Default::default()
		},
	)
	.await?;
	let responsible =
		SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, responsible_id).await?;
	assert_eq!(responsible.sender_presave_id, sender_id);
	assert_eq!(responsible.department.as_deref(), Some("PV"));
	assert_eq!(responsible.person_given_name.as_deref(), Some("After"));
	assert_audit_changed_field(
		&mm,
		"sender_presave_responsible_persons",
		responsible_id,
		"person_given_name",
		json!("Before"),
		json!("After"),
	)
	.await?;
	assert_eq!(
		SenderPresaveResponsiblePersonBmc::list_by_parent(&ctx, &mm, sender_id)
			.await?[0]
			.id,
		responsible_id
	);

	let consignee_id = ReceiverPresaveConsigneeBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveConsigneeForCreate {
			receiver_presave_id: receiver_id,
			sequence_number: 1,
			name: Some("Consignee Before".into()),
			phone: None,
			email: Some("before@example.com".into()),
		},
	)
	.await?;
	ReceiverPresaveConsigneeBmc::update(
		&ctx,
		&mm,
		consignee_id,
		ReceiverPresaveConsigneeForUpdate {
			name: Some("Consignee After".into()),
			..Default::default()
		},
	)
	.await?;
	let consignee =
		ReceiverPresaveConsigneeBmc::get(&ctx, &mm, consignee_id).await?;
	assert_eq!(consignee.receiver_presave_id, receiver_id);
	assert_eq!(consignee.name.as_deref(), Some("Consignee After"));
	assert_audit_changed_field(
		&mm,
		"receiver_presave_consignees",
		consignee_id,
		"name",
		json!("Consignee Before"),
		json!("Consignee After"),
	)
	.await?;
	assert_eq!(
		ReceiverPresaveConsigneeBmc::list_by_parent(&ctx, &mm, receiver_id).await?
			[0]
		.id,
		consignee_id
	);

	let substance_id = ProductPresaveActiveSubstanceBmc::create(
		&ctx,
		&mm,
		ProductPresaveActiveSubstanceForCreate {
			product_presave_id: product_id,
			sequence_number: 1,
			substance_name: Some("Substance Before".into()),
			substance_termid_version: None,
			substance_termid: Some("SUB-1".into()),
			mfds_version: None,
			mfds_id: None,
			strength_value: Some(Decimal::new(125, 2)),
			strength_unit: Some("mg".into()),
		},
	)
	.await?;
	ProductPresaveActiveSubstanceBmc::update(
		&ctx,
		&mm,
		substance_id,
		ProductPresaveActiveSubstanceForUpdate {
			substance_name: Some("Substance After".into()),
			..Default::default()
		},
	)
	.await?;
	let substance =
		ProductPresaveActiveSubstanceBmc::get(&ctx, &mm, substance_id).await?;
	assert_eq!(substance.product_presave_id, product_id);
	assert_eq!(substance.substance_name.as_deref(), Some("Substance After"));
	assert_audit_changed_field(
		&mm,
		"product_presave_active_substances",
		substance_id,
		"substance_name",
		json!("Substance Before"),
		json!("Substance After"),
	)
	.await?;
	assert_eq!(
		ProductPresaveActiveSubstanceBmc::list_by_parent(&ctx, &mm, product_id)
			.await?[0]
			.id,
		substance_id
	);

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
	StudyPresaveRegistrationNumberBmc::update(
		&ctx,
		&mm,
		registration_id,
		StudyPresaveRegistrationNumberForUpdate {
			registration_number: Some("REG-after".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let registration =
		StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, registration_id).await?;
	assert_eq!(registration.study_presave_id, study_id);
	assert_eq!(
		registration.registration_number.as_deref(),
		Some("REG-after")
	);
	assert!(registration.deleted);
	assert_audit_changed_field(
		&mm,
		"study_presave_registration_numbers",
		registration_id,
		"registration_number",
		json!("REG-before"),
		json!("REG-after"),
	)
	.await?;
	assert_eq!(
		StudyPresaveRegistrationNumberBmc::list_by_parent(&ctx, &mm, study_id)
			.await?[0]
			.id,
		registration_id
	);

	let cross_reported_ind_id = StudyPresaveFdaCrossReportedIndNumberBmc::create(
		&ctx,
		&mm,
		StudyPresaveFdaCrossReportedIndNumberForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			ind_number: Some("IND-123".into()),
			ind_number_null_flavor: None,
			deleted: Some(false),
		},
	)
	.await?;
	StudyPresaveFdaCrossReportedIndNumberBmc::update(
		&ctx,
		&mm,
		cross_reported_ind_id,
		StudyPresaveFdaCrossReportedIndNumberForUpdate {
			ind_number: Some("IND-456".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let cross_reported_ind = StudyPresaveFdaCrossReportedIndNumberBmc::get(
		&ctx,
		&mm,
		cross_reported_ind_id,
	)
	.await?;
	assert_eq!(cross_reported_ind.ind_number.as_deref(), Some("IND-456"));
	assert!(cross_reported_ind.deleted);
	assert_eq!(
		StudyPresaveFdaCrossReportedIndNumberBmc::list_by_parent(
			&ctx, &mm, study_id
		)
		.await?[0]
			.id,
		cross_reported_ind_id
	);

	let study_product_id = StudyPresaveProductBmc::create(
		&ctx,
		&mm,
		StudyPresaveProductForCreate {
			study_presave_id: study_id,
			sequence_number: 2,
			product_presave_id: Some(product_id),
			product_name: Some("STUDY-PRODUCT-before".into()),
			deleted: Some(false),
		},
	)
	.await?;
	StudyPresaveProductBmc::update(
		&ctx,
		&mm,
		study_product_id,
		StudyPresaveProductForUpdate {
			product_name: Some("STUDY-PRODUCT-after".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let study_product =
		StudyPresaveProductBmc::get(&ctx, &mm, study_product_id).await?;
	assert_eq!(study_product.study_presave_id, study_id);
	assert_eq!(
		study_product.product_name.as_deref(),
		Some("STUDY-PRODUCT-after")
	);
	assert!(study_product.deleted);
	assert_audit_changed_field(
		&mm,
		"study_presave_products",
		study_product_id,
		"product_name",
		json!("STUDY-PRODUCT-before"),
		json!("STUDY-PRODUCT-after"),
	)
	.await?;
	assert_eq!(
		StudyPresaveProductBmc::list_by_parent(&ctx, &mm, study_id).await?[0].id,
		study_product_id
	);

	StudyPresaveProductBmc::delete(&ctx, &mm, study_product_id).await?;
	StudyPresaveFdaCrossReportedIndNumberBmc::delete(
		&ctx,
		&mm,
		cross_reported_ind_id,
	)
	.await?;
	StudyPresaveRegistrationNumberBmc::delete(&ctx, &mm, registration_id).await?;
	ProductPresaveActiveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	ReceiverPresaveConsigneeBmc::delete(&ctx, &mm, consignee_id).await?;
	SenderPresaveResponsiblePersonBmc::delete(&ctx, &mm, responsible_id).await?;
	SenderPresaveGatewayBmc::delete(&ctx, &mm, gateway_first_id).await?;
	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, fda_product_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "requires receiver presave route seed data"]
async fn test_seeded_receiver_presave_routes_match_reference_labels() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;

	let routes: Vec<(
		String,
		String,
		String,
		String,
		String,
		String,
		String,
		Option<String>,
		String,
	)> = sqlx::query_as(
		"SELECT
			p.organization_name,
			r.authority,
			r.receiver_label,
			r.condition_page,
			r.condition_field_code,
			r.condition_value_code,
			r.condition_value_label,
			r.batch_receiver_identifier,
			r.message_receiver_identifier
		 FROM receiver_presave_routes r
		 JOIN receiver_presaves p ON p.id = r.receiver_presave_id
		 WHERE p.organization_id = $1
		   AND p.organization_name = ANY($2)
		 ORDER BY r.authority, r.receiver_label",
	)
	.bind(demo_org_id())
	.bind(&["MFDS", "FDA"])
	.fetch_all(mm.dbx().db())
	.await?;

	let labels: HashSet<&str> = routes
		.iter()
		.map(|(_, _, receiver_label, _, _, _, _, _, _)| receiver_label.as_str())
		.collect();
	for expected in [
		"MFDS(CT)",
		"MFDS(CU)",
		"MFDS(KR)",
		"MFDS(FR)",
		"MFDS(CF)",
		"FDA(CDER IND)",
		"FDA(CDER IND-exempt BA/BE)",
		"FDA(CBER IND)",
		"FDA(Postmarket)",
	] {
		assert!(labels.contains(expected), "missing seeded route {expected}");
	}
	assert_eq!(
		labels.len(),
		9,
		"seeded receiver route labels should match the reference set"
	);

	let fda_cber_ind = routes
		.iter()
		.find(|(_, _, receiver_label, _, _, _, _, _, _)| {
			receiver_label == "FDA(CBER IND)"
		})
		.expect("FDA(CBER IND) route should be seeded");
	assert_eq!(
		fda_cber_ind,
		&(
			"FDA".to_string(),
			"fda".to_string(),
			"FDA(CBER IND)".to_string(),
			"CI".to_string(),
			"FDA_REPORT_TYPE".to_string(),
			"3".to_string(),
			"CBER IND".to_string(),
			Some("ZZFDA_PREMKT".to_string()),
			"CBER_IND".to_string(),
		)
	);

	let fda_postmarket = routes
		.iter()
		.find(|(_, _, receiver_label, _, _, _, _, _, _)| {
			receiver_label == "FDA(Postmarket)"
		})
		.expect("FDA(Postmarket) route should be seeded");
	assert_eq!(
		fda_postmarket,
		&(
			"FDA".to_string(),
			"fda".to_string(),
			"FDA(Postmarket)".to_string(),
			"CI".to_string(),
			"FDA_REPORT_TYPE".to_string(),
			"4".to_string(),
			"Postmarket".to_string(),
			Some("ZZFDA".to_string()),
			"CDER".to_string(),
		)
	);

	let mfds_ct = routes
		.iter()
		.find(|(_, _, receiver_label, _, _, _, _, _, _)| {
			receiver_label == "MFDS(CT)"
		})
		.expect("MFDS(CT) route should be seeded");
	assert_eq!(
		mfds_ct,
		&(
			"MFDS".to_string(),
			"mfds".to_string(),
			"MFDS(CT)".to_string(),
			"CI".to_string(),
			"MFDS_REPORT_TYPE".to_string(),
			"1".to_string(),
			"임상시험계획의 승인을 받은 자".to_string(),
			Some("MFDS-O-CT".to_string()),
			"MFDS-O-CT".to_string(),
		)
	);

	let mfds_kr = routes
		.iter()
		.find(|(_, _, receiver_label, _, _, _, _, _, _)| {
			receiver_label == "MFDS(KR)"
		})
		.expect("MFDS(KR) route should be seeded");
	assert_eq!(
		mfds_kr,
		&(
			"MFDS".to_string(),
			"mfds".to_string(),
			"MFDS(KR)".to_string(),
			"CI".to_string(),
			"MFDS_REPORT_TYPE".to_string(),
			"3".to_string(),
			"시판 후 이상사례 국내보고".to_string(),
			Some("MFDS-O-KR".to_string()),
			"MFDS-O-KR".to_string(),
		)
	);

	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "requires receiver presave route schema"]
async fn test_receiver_presave_route_bmc_crud() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Route Receiver Org {suffix}")),
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

	let route_id = ReceiverPresaveRouteBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveRouteForCreate {
			receiver_presave_id: receiver_id,
			sequence_number: 1,
			authority: "fda".into(),
			receiver_label: "FDA(CBER IND)".into(),
			batch_receiver_identifier: Some("FDA-CBER-BATCH".into()),
			message_receiver_identifier: "FDA-CBER-MESSAGE".into(),
			condition_page: "C.5.4".into(),
			condition_field_code: "C.5.4.r.1".into(),
			condition_operator: "Equal".into(),
			condition_value_code: "IND".into(),
			condition_value_label: "Investigational New Drug".into(),
		},
	)
	.await?;

	let routes =
		ReceiverPresaveRouteBmc::list_by_parent(&ctx, &mm, receiver_id).await?;
	assert_eq!(routes.len(), 1);
	assert_eq!(routes[0].id, route_id);
	assert_eq!(routes[0].receiver_presave_id, receiver_id);
	assert_eq!(routes[0].receiver_label, "FDA(CBER IND)");
	assert_eq!(
		routes[0].batch_receiver_identifier.as_deref(),
		Some("FDA-CBER-BATCH")
	);
	assert_eq!(routes[0].message_receiver_identifier, "FDA-CBER-MESSAGE");

	ReceiverPresaveRouteBmc::update(
		&ctx,
		&mm,
		route_id,
		ReceiverPresaveRouteForUpdate {
			sequence_number: Some(2),
			batch_receiver_identifier: Some("FDA-CBER-BATCH-2".into()),
			..Default::default()
		},
	)
	.await?;

	let route = ReceiverPresaveRouteBmc::get(&ctx, &mm, route_id).await?;
	assert_eq!(route.sequence_number, 2);
	assert_eq!(
		route.batch_receiver_identifier.as_deref(),
		Some("FDA-CBER-BATCH-2")
	);
	assert_eq!(route.authority, "fda");
	assert_eq!(route.condition_operator, "Equal");

	ReceiverPresaveRouteBmc::delete(&ctx, &mm, route_id).await?;
	assert!(
		ReceiverPresaveRouteBmc::list_by_parent(&ctx, &mm, receiver_id)
			.await?
			.is_empty()
	);
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;

	Ok(())
}
