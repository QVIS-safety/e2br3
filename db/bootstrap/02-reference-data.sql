-- Canonical initial data from a fresh, unseeded application database.
-- Includes the internal system principal, not interactive users or customer data.
--
-- PostgreSQL database dump
--



SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;

--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.organizations (id, name, org_type, address, city, state, postcode, country_code, contact_email, contact_phone, active, created_at, updated_at, created_by, updated_by) VALUES ('00000000-0000-0000-0000-000000000000', 'System', 'Internal', NULL, NULL, NULL, NULL, 'XX', NULL, NULL, true, '2026-11-27 12:32:27.950027+00', '2026-11-27 12:32:27.950027+00', '00000000-0000-0000-0000-000000000001', NULL);


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.users (id, organization_id, email, username, pwd, pwd_salt, token_salt, role, comments, other_information, access_start_at, access_end_at, access_sender_ids, access_product_ids, access_study_ids, access_blind_allowed, active_sender_identifier, active, must_change_password, last_login_at, created_at, updated_at, created_by, updated_by) VALUES ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000000', 'system@e2br3.local', 'system', NULL, 'b04d73dd-1217-4137-903a-7dae0de9da39', '171a9c64-adb2-49d7-a92b-280126671b9e', 'system_admin', NULL, NULL, NULL, NULL, NULL, NULL, NULL, false, NULL, true, false, NULL, '2026-11-27 12:32:27.950027+00', '2026-11-27 12:32:29.946508+00', NULL, NULL);


--
-- Data for Name: app_settings; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.app_settings (organization_id, key, value, updated_at, updated_by) VALUES ('00000000-0000-0000-0000-000000000000', 'system', '{"notation": false, "timezone": "Asia/Seoul", "workflow": {"statuses": [{"name": "Saved", "due_days": 0, "editable": true, "description": "Default state", "allowed_roles": ["sponsor_admin_cro"]}]}, "appendices": ["ICH"], "idf_version": "3.0", "orientation": "Landscape", "data_ordering": "Primary data will appear first", "meddra_version": "28.1", "meddra_language": "English", "workflow_enabled": false, "import_date_update": {"date_of_creation": false, "most_recent_info_date": false, "report_first_received_date": false}, "case_number_padding": 6, "case_number_setting": "AE Row No.", "idle_session_minutes": 60, "case_number_identifier": "ICSR", "session_warning_minutes": 5, "case_number_format_fields": ["AE Row No."], "apply_comments_on_exported_xml": false, "case_number_sequence_condition": "Per sender", "apply_sender_info_to_imported_cases": false, "upload_excel_template_without_element_label": false}', '2026-11-27 12:32:29.743442+00', NULL);


--
-- Data for Name: e2b_code_lists; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (1, 'dc96f121-53d9-47f7-8221-6f986f613ffd', 'report_type', '1', 'Spontaneous report', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (2, 'c937a390-3d2a-40f4-8aa8-98b3f72add32', 'report_type', '2', 'Report from study', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (3, 'c6f814ff-2a2d-4ce8-b2a7-e5444184c69b', 'report_type', '3', 'Other', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (4, '75da9ced-41f4-48be-b3a2-3838c26d6e9b', 'report_type', '4', 'Not available to sender', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (5, '7e2177c9-0a68-4e1b-8329-5b19a16f43ef', 'sender_type', '1', 'Pharmaceutical company', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (6, 'a11593fc-abf3-4adb-bbe0-5f0a81452919', 'sender_type', '2', 'Regulatory authority', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (7, '1d69726e-bd00-41a3-b270-5b8db8c17b37', 'sender_type', '3', 'Health professional', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (8, 'c2fba314-a1e3-42cd-afd8-6e3f9caeada6', 'sender_type', '4', 'Regional pharmacovigilance center', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (9, '9675f2e6-8c22-462a-837b-51eab2afa7b6', 'sender_type', '5', 'WHO collaborating center for international drug monitoring', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (10, '278c280b-f2e0-4b09-b19a-478f80007cf1', 'sender_type', '6', 'Other', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (11, 'e585c6f0-8f75-4d7d-9324-f6de7776b7e5', 'sender_type', '7', 'Patient / Consumer', NULL, 7, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (12, '8c8abe4b-8329-4d17-9602-bdfc673008bb', 'qualification', '1', 'Physician', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (13, 'be7980b2-ee64-459a-b399-bdba05e4766d', 'qualification', '2', 'Pharmacist', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (14, '23c80937-dd63-403c-8876-ba34e382ec3a', 'qualification', '3', 'Other health professional', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (15, '21ef0f9d-e46c-4e11-a0a0-a4648816abe1', 'qualification', '4', 'Lawyer', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (16, '159e85af-3f4b-4264-88ce-01154f551108', 'qualification', '5', 'Consumer or other non health professional', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (17, '4f7708c6-4958-412c-beb8-15076fd623dc', 'sex', '0', 'Unknown', NULL, 0, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (18, '10187873-b554-4812-a0bb-6d64cff3f251', 'sex', '1', 'Male', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (19, '41d1b4e4-ee3f-4d75-aa1c-5f5b243cfbfd', 'sex', '2', 'Female', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (21, '14066895-66ee-4045-a40b-34b791964fd3', 'age_group', '1', 'Neonate', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (22, '594f32f7-20cf-4965-bf21-02fd6efe9cb9', 'age_group', '2', 'Infant', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (23, 'ad771ee7-a40b-4632-b326-a8bf3bc945e6', 'age_group', '3', 'Child', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (24, 'a9ad53e1-c210-4fea-9e81-6e446e815611', 'age_group', '4', 'Adolescent', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (25, 'f426c4d2-40e8-45b1-8e4f-34fbdcc631a7', 'age_group', '5', 'Adult', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (26, '7566b6b6-3711-4927-abfa-30c85e20865c', 'age_group', '6', 'Elderly', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (27, '0a038b9f-e139-4fda-8c5b-f1b729dc338b', 'age_unit', '800', 'Decade', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (28, 'dc443f26-be03-44f3-92cd-c2a6bad67f0d', 'age_unit', '801', 'Year', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (29, '4ca456a1-4edf-4c98-bef3-67d9d7901ee2', 'age_unit', '802', 'Month', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (30, '763eadbf-626b-4441-a295-84b032a9c758', 'age_unit', '803', 'Week', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (31, '5daa6fe0-dc3f-4f7a-a1be-c7e4eeaac96f', 'age_unit', '804', 'Day', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (32, '5bb0df9b-7ac7-456d-beac-32b0e5e7ecef', 'age_unit', '805', 'Hour', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (33, '5fb43ecb-adb9-44db-aad6-3338fb787d18', 'reaction_outcome', '0', 'Unknown', NULL, 0, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (34, 'fcfe5916-93dd-4922-8e0e-6f5a712a22ab', 'reaction_outcome', '1', 'Recovered/resolved', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (35, '36b48a0a-5d98-45ff-b3ce-a851a8055d00', 'reaction_outcome', '2', 'Recovering/resolving', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (36, '12d851df-e0b3-4705-a69f-ede53d6022a0', 'reaction_outcome', '3', 'Not recovered/not resolved', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (37, 'c0dce31e-7937-4400-82fc-724b8fd2850a', 'reaction_outcome', '4', 'Recovered/resolved with sequelae', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (38, 'b90a6cf5-b184-4c07-9c96-5119ae724d5c', 'reaction_outcome', '5', 'Fatal', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (39, '018c37fe-6548-40d6-ae94-8dc0f326d7f4', 'drug_characterization', '1', 'Suspect', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (40, '5ed0f7a9-77e6-4eb1-b76f-d6656d5fb9f2', 'drug_characterization', '2', 'Concomitant', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (41, '52dd0090-440f-4451-b827-6d026c455ae1', 'drug_characterization', '3', 'Interacting', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (43, '8f8c7d0a-8adb-4c33-9bd7-bb7c25fa36f7', 'drug_action', '1', 'Drug withdrawn', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (44, '47f2444b-42f9-43d0-b3e2-0757151d41a9', 'drug_action', '2', 'Dose reduced', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (45, 'fd1190ab-6550-490b-858f-3b4547e5d76f', 'drug_action', '3', 'Dose increased', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (46, '301c3f02-31c7-4636-afab-9cec16166372', 'drug_action', '4', 'Dose not changed', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (47, 'd19f2ae1-94ff-425e-958c-97ae6e381bde', 'drug_action', '5', 'Unknown', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (48, '8969f8a5-a42e-4e03-a0e3-14cb5e7b8e66', 'drug_action', '6', 'Not applicable', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (49, '80024130-ead0-4356-8509-78b8dbde44f1', 'rechallenge', '1', 'Yes, reaction recurred', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (50, '3f3bda40-4c60-47b0-a54a-b0b919facf9e', 'rechallenge', '2', 'Yes, reaction did not recur', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (51, 'e517238f-2fcd-4710-b65d-99e64cf492d1', 'rechallenge', '3', 'No', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (52, 'cee54166-c24f-43f5-bff3-f81fce9cce75', 'rechallenge', '4', 'Unknown', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (53, 'c05a7ed5-2a07-4f65-ad2f-26c179ca266c', 'primary_source_regulatory', '1', 'Yes', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (54, '92908699-e2c5-4b47-b821-faa7bef3f37f', 'primary_source_regulatory', '2', 'No', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (55, '854e0302-9dd9-4310-a6b2-0b21c889d8e8', 'study_type', '1', 'Clinical trial', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (56, '2c1168b8-6a8b-4ade-b072-afe15be8efe6', 'study_type', '2', 'Individual patient use', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (57, '6cdd9d29-2332-40fa-b639-f949d4ee7961', 'study_type', '3', 'Other studies', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (58, 'e212c7f7-2e2e-449d-8bba-59bafe38b46c', 'term_highlighted', '1', 'Yes, highlighted by reporter, NOT serious', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (59, '747ebbaa-767d-4bd9-b4a8-c4111677cb3a', 'term_highlighted', '2', 'No, not highlighted by reporter, NOT serious', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (60, '01202add-b77b-4fd7-afed-6ca93f811e5d', 'term_highlighted', '3', 'Yes, highlighted by reporter, SERIOUS', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (61, 'f1364505-e962-4ae4-a220-a2f404f5a9d3', 'term_highlighted', '4', 'No, not highlighted by reporter, SERIOUS', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (62, '96f202e4-0619-4b80-8122-ded08c503d6c', 'medical_confirmation', '1', 'Yes', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (63, 'c6979b97-5cca-49b4-b2ea-cbcafd726996', 'medical_confirmation', '2', 'No', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (64, '3d9a8928-48e9-49fe-9b88-e442be783bac', 'medical_confirmation', '3', 'Unknown', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (65, '5cfc1e12-68a7-4a8e-86f6-88c46be733d9', 'receiver_type', '1', 'Pharmaceutical company', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (66, '15f7a710-0c7a-472c-933b-1c052cbd32e7', 'receiver_type', '2', 'Regulatory authority', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (67, '5d7bc5f4-bea5-4b2e-a4a2-cedabdd1db2e', 'receiver_type', '3', 'Health professional', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (68, '2e37083a-5490-4e2a-8482-f523bc1d5117', 'receiver_type', '4', 'Regional pharmacovigilance center', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (69, 'db900dc8-9944-49cb-9980-5dec808a3809', 'receiver_type', '5', 'WHO collaborating center for international drug monitoring', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (70, '99ac7759-24fe-47fe-add2-efcdef3c72a7', 'receiver_type', '6', 'Other', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (71, '1cc9405c-0dfa-4155-8cdb-fbad3b825c1d', 'recurrence_action', '1', 'Drug readministered', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (72, '2242cd89-44c5-4c26-8ae7-24a182d10ee6', 'recurrence_action', '2', 'Drug not readministered', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (73, 'dc443845-73a8-46e2-bebb-b835ac57c739', 'recurrence_action', '3', 'Unknown', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (74, 'f6bbb8c7-8710-4305-a1c0-d5501c27fd22', 'recurrence_action', '4', 'Not applicable', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (75, 'bf01da5d-3a60-4bc9-bf07-5b7f9704ec47', 'reaction_recurred', '1', 'Yes', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (76, '1d5748ed-bb03-4f85-84b1-a04fb4cb49e0', 'reaction_recurred', '2', 'No', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (77, '2e9c3ee5-b22f-4614-9d9a-44236811c3ae', 'reaction_recurred', '3', 'Unknown', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (78, 'b5c4ef52-7ef9-4d82-ad33-92a18ddacab2', 'dechallenge_result', '1', 'Yes', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (79, 'c7828aaa-68c5-4b6e-911c-8e6943162783', 'dechallenge_result', '2', 'No', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (80, '624d86b8-b663-4446-a0cb-a34e80edb23b', 'dechallenge_result', '3', 'Not applicable', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (81, 'fe78db61-a243-402d-aba9-9af6e1a30154', 'null_flavor', 'NI', 'No Information', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (82, 'a01e0c2a-a794-4286-b861-122006c1ee94', 'null_flavor', 'UNK', 'Unknown', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (83, 'da1dfd64-8144-48a2-a1ce-2461095a6c23', 'null_flavor', 'ASKU', 'Asked but Unknown', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (84, '3d3e74a2-3006-4fc8-b6af-fcb3343a8067', 'null_flavor', 'NASK', 'Not Asked', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (85, '0c46d53c-c460-43df-afa7-48b6d6879f31', 'null_flavor', 'MSK', 'Masked', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (86, '21376cd8-da78-4ee3-a5f4-70ed44a76d14', 'route_of_administration', '001', 'Auricular (otic)', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (87, '57c6144b-815f-46c1-826d-1dcd3f742f22', 'route_of_administration', '002', 'Buccal', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (88, 'bf550726-9b61-4a12-9232-47011afcdeba', 'route_of_administration', '003', 'Cutaneous', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (89, 'b5ac81fd-8782-42e7-86e7-273058447d93', 'route_of_administration', '004', 'Dental', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (42, 'f13c7587-6dc5-4f30-9b0a-5a8a50116424', 'drug_characterization', '4', 'Drug Not Administered', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (90, 'bcad9d38-ca4c-4256-a905-8cb56bbe2e78', 'route_of_administration', '005', 'Endocervical', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (91, '80b3337e-59f3-4f7f-9a99-87d5ecb0374c', 'route_of_administration', '006', 'Endosinusial', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (92, '29a987ea-4fd3-4a63-9036-938a94a761d5', 'route_of_administration', '007', 'Endotracheal', NULL, 7, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (93, 'ca10dbbe-d3fa-4626-8c14-ec4265348334', 'route_of_administration', '008', 'Epidural', NULL, 8, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (94, '1281b2c2-5f55-4e9b-b357-d800988fac12', 'route_of_administration', '009', 'Extra-amniotic', NULL, 9, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (95, '23424c6c-096e-4006-8528-cc80dcea6aa0', 'route_of_administration', '010', 'Hemodialysis', NULL, 10, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (96, '723bf445-e084-46c9-a9a3-63df8676ae73', 'route_of_administration', '011', 'Intra-corporus cavernosum', NULL, 11, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (97, '756a0001-7eaf-491c-96a3-36d8fbe6ccaf', 'route_of_administration', '012', 'Intra-amniotic', NULL, 12, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (98, 'a44a3220-d901-44d1-a3b3-a7bb73b1e71f', 'route_of_administration', '013', 'Intra-arterial', NULL, 13, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (99, '36fce46d-2106-433f-821e-cb7cb0d62edd', 'route_of_administration', '014', 'Intra-articular', NULL, 14, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (100, '217a0323-6bac-4c51-917a-64c2e1ecc5d3', 'route_of_administration', '015', 'Intra-uterine', NULL, 15, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (101, '7b050216-9a90-41cc-97db-77dc4409c2fd', 'route_of_administration', '016', 'Intracardiac', NULL, 16, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (102, '5a788d41-0011-44ff-98ad-35dbcbba6cb3', 'route_of_administration', '017', 'Intracavernous', NULL, 17, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (103, 'a1ca40ef-cd88-4296-9b3b-26271925b56e', 'route_of_administration', '018', 'Intracerebral', NULL, 18, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (104, 'e24ed4ec-04ee-4f5e-8545-9e4f1b32a0f8', 'route_of_administration', '019', 'Intracervical', NULL, 19, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (105, 'b6cb3c72-de74-485b-a5a4-62c03a1781a7', 'route_of_administration', '020', 'Intracisternal', NULL, 20, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (106, 'a438e981-8d29-486e-be42-5309539f70f5', 'route_of_administration', '021', 'Intracorneal', NULL, 21, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (107, '042a2b90-9525-46b5-ab74-c6c7f82313ba', 'route_of_administration', '022', 'Intracoronary', NULL, 22, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (108, '867fac0e-e7db-439c-aa44-211f2e6af64e', 'route_of_administration', '023', 'Intradermal', NULL, 23, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (109, 'c6e3b98d-46ba-4a73-8161-f6646d622bda', 'route_of_administration', '024', 'Intradiscal', NULL, 24, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (110, '1d0874a5-3bd6-499a-8500-16d03e1915f1', 'route_of_administration', '025', 'Intrahepatic', NULL, 25, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (111, '95be0dff-e6cd-4011-abe1-9aee3a02475a', 'route_of_administration', '026', 'Intralesional', NULL, 26, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (112, '285491e8-faed-49f4-b37e-52ce87348967', 'route_of_administration', '027', 'Intralymphatic', NULL, 27, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (113, '7dcd04f1-7f2c-491b-86cb-2c03a0890429', 'route_of_administration', '028', 'Intramedullar (bone marrow)', NULL, 28, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (114, '14d295bd-11c1-4371-a7f3-34489725cce4', 'route_of_administration', '029', 'Intrameningeal', NULL, 29, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (115, '5bd243af-8627-4771-9a25-d9289c060ac0', 'route_of_administration', '030', 'Intramuscular', NULL, 30, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (116, 'c03ac60a-f725-4558-9fef-723b11fe5761', 'route_of_administration', '031', 'Intraocular', NULL, 31, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (117, '35f1c5c5-ec59-4187-a8d6-85720a933a67', 'route_of_administration', '032', 'Intrapericardial', NULL, 32, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (118, '5804a710-7c54-44d3-9ba6-6130d0f5c726', 'route_of_administration', '033', 'Intraperitoneal', NULL, 33, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (119, '93dee490-67f9-43ba-876a-c3417665f555', 'route_of_administration', '034', 'Intrapleural', NULL, 34, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (120, '30858825-1685-4b0f-8061-9f7b258576d7', 'route_of_administration', '035', 'Intrasynovial', NULL, 35, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (121, '44db8315-ebbb-4106-95b7-59726633d956', 'route_of_administration', '036', 'Intrathecal', NULL, 36, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (122, 'd01bc897-0b0a-4a5c-9b91-8a7756ed4541', 'route_of_administration', '037', 'Intrathoracic', NULL, 37, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (123, '729b62e5-d2aa-4377-b6b3-33b5b91e2ebe', 'route_of_administration', '038', 'Intratracheal', NULL, 38, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (124, '3275ac22-7201-4243-b45b-4b6b8965f957', 'route_of_administration', '039', 'Intratumour', NULL, 39, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (125, '2c45366b-066b-4f9e-981c-54d2701c74d5', 'route_of_administration', '040', 'Intratympanic', NULL, 40, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (126, '0054ecc8-d7d4-43cc-bea8-a67179171bae', 'route_of_administration', '041', 'Intravenous (not elsewhere classified)', NULL, 41, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (127, '859450ad-4830-432d-b0f2-c91c65e4a50e', 'route_of_administration', '042', 'Intravenous bolus', NULL, 42, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (128, '9deee2a5-256b-4470-b727-da6d88ab90dd', 'route_of_administration', '043', 'Intravenous drip', NULL, 43, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (129, 'fb2c5c05-5fbe-49c9-8150-8c90880c79de', 'route_of_administration', '044', 'Intravesical', NULL, 44, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (130, '27e44f68-61be-4bca-943b-c2d3d3a34e42', 'route_of_administration', '045', 'Iontophoresis', NULL, 45, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (131, '40418084-4fad-4b4d-a941-d6988b0e34d6', 'route_of_administration', '046', 'Nasal', NULL, 46, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (132, '065f0afe-e93d-4c07-b2f7-9777c4b4f745', 'route_of_administration', '047', 'Ocular', NULL, 47, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (133, '86e9630e-21d5-48ef-951e-3e114b1730e2', 'route_of_administration', '048', 'Oral', NULL, 48, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (134, 'ab5d5de4-e29c-4b64-8c7d-d657d2e6ed22', 'route_of_administration', '049', 'Oropharyngeal', NULL, 49, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (135, 'fea5b23b-22d4-4cd9-80fc-3eb1cfff6719', 'route_of_administration', '050', 'Other', NULL, 50, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (136, '5be3fe45-dd84-443c-b2ca-a3b1f1f86d43', 'route_of_administration', '051', 'Parenteral', NULL, 51, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (137, '02343db2-d813-49b2-8e4e-3653f8af2007', 'route_of_administration', '052', 'Periarticular', NULL, 52, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (138, '84aa6360-a491-4624-9342-d2b7b4cb390f', 'route_of_administration', '053', 'Perineural', NULL, 53, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (139, 'ba5de4c3-6562-42f8-9b1f-6b132b45e407', 'route_of_administration', '054', 'Rectal', NULL, 54, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (140, 'd83cd559-dccd-4cb4-a273-b103d2fa86bf', 'route_of_administration', '055', 'Respiratory (inhalation)', NULL, 55, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (141, '96b9fa73-4d65-4df4-bb7e-2669cffe16c5', 'route_of_administration', '056', 'Retrobulbar', NULL, 56, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (142, '8a7a2be6-a347-4607-ab1b-4034f0a3049b', 'route_of_administration', '057', 'Sunconjunctival', NULL, 57, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (143, 'b4a25026-0692-4b28-8f43-b2dc3d2fdfff', 'route_of_administration', '058', 'Subcutaneous', NULL, 58, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (144, '6fccb5a7-a697-4471-a530-2b0828d2fb71', 'route_of_administration', '059', 'Subdermal', NULL, 59, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (145, 'de5b099d-908b-4cf2-9e8f-0c9360922ac6', 'route_of_administration', '060', 'Sublingual', NULL, 60, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (146, 'ba4c0e95-a689-45ca-bfbd-125c709f9689', 'route_of_administration', '061', 'Topical', NULL, 61, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (147, 'a78d011f-22b0-4c80-b4dd-47a32e672561', 'route_of_administration', '062', 'Transdermal', NULL, 62, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (148, 'bb9ed3dd-1a64-427d-be27-b61f68941548', 'route_of_administration', '063', 'Transmammary', NULL, 63, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (149, 'f3149456-2721-433d-a2d4-a07c2e134d9f', 'route_of_administration', '064', 'Transplacental', NULL, 64, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (150, '891c3781-1407-47db-9b8b-7022e44d5bb0', 'route_of_administration', '065', 'Unknown', NULL, 65, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (151, '009a49a7-2771-4808-ad01-d4396b69328e', 'route_of_administration', '066', 'Urethral', NULL, 66, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (152, '0c0f0d53-9a0d-41a0-9f73-238c4d1b9e48', 'route_of_administration', '067', 'Vaginal', NULL, 67, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (153, '6f279aa2-ab6b-4f15-a9b1-92f6166d4e78', 'fda_g_k_10a_additional_info', '1', 'Test', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (154, '8cf49758-9266-4f86-a97d-c59c568c2c74', 'fda_g_k_10a_additional_info', '2', 'Reference', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (155, '0a8bb435-5629-4b62-9412-dc9b4dd02bf4', 'fda_g_k_10a_additional_info', '3', 'Bulk ingredient', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (156, '9ae76728-4a2a-46c6-b6fe-2bd115818090', 'fda_g_k_10a_additional_info', '4', 'Bulk Ingredient For Human Prescription Compounding', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (157, '3719f13b-7697-4a4b-8466-85a8e03d5154', 'fda_g_k_10a_additional_info', '5', 'Unapproved Drug Product Manufactured Exclusively for Private Label Distributer', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (158, 'e28436fe-300b-488a-bc4e-befc146c715e', 'fda_g_k_10_1_specialized_product_category', 'C102834', 'Type 1: Convenience Kit of Co-Package', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (159, '53b95240-af8b-4d50-9b6e-e65635655011', 'fda_g_k_10_1_specialized_product_category', 'C102835', 'Type 2: Prefilled Drug Delivery Device/System (syringe, patch, etc.)', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (160, '7a48bd16-38c5-4366-afc8-d37c9760f225', 'fda_g_k_10_1_specialized_product_category', 'C102836', 'Type 3: Prefilled Biologic Delivery Device/System (syringe, patch, etc.)', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (161, '52b540fe-ed88-46ee-952d-5afb475bb865', 'fda_g_k_10_1_specialized_product_category', 'C102837', 'Type 4: Device Coated/Impregnated/Otherwise Combined with Drug', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (162, '5ee4771b-6fa5-49bc-983d-8dceaa170c07', 'fda_g_k_10_1_specialized_product_category', 'C102838', 'Type 5: Device Coated or Otherwise Combined with Biologic', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (163, '2e22a6d5-116d-48ce-87aa-b9b3893ec6b9', 'fda_g_k_10_1_specialized_product_category', 'C102839', 'Type 6: Drug/Biologic Combination', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (164, 'd55adc9c-8c0e-4f44-ba22-feb907f9a644', 'fda_g_k_10_1_specialized_product_category', 'C102840', 'Type 7: Separate Products Requiring Cross Labeling', NULL, 7, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (165, '86f7624a-047a-448e-81c4-4f5ac6dc2674', 'fda_g_k_10_1_specialized_product_category', 'C102841', 'Type 8: Possible Combination Based on Cross Labeling of Separate Products (Temporary Type)', NULL, 8, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (166, '6a1e7f08-1599-4254-b863-32b56abc67d5', 'fda_g_k_10_1_specialized_product_category', 'C102842', 'Type 9: Other Type of Part 3 Combination Product (e.g., Drug/Device/Biological Product)', NULL, 9, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (167, 'a532948f-b763-467a-b1cc-af3bf39cfc80', 'fda_g_k_12_r_2_r_follow_up_type', '1', 'Correction', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (168, '8f1030b7-365f-49ce-b88b-748d78445570', 'fda_g_k_12_r_2_r_follow_up_type', '2', 'Additional Information', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (169, '38e71b4e-d1b0-429b-88d7-363a7acef39f', 'fda_g_k_12_r_2_r_follow_up_type', '3', 'Response to FDA Request', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (170, '509de95a-7a1f-4fbf-b069-20e376c3dc83', 'fda_g_k_12_r_2_r_follow_up_type', '4', 'Device Evaluation', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (171, '7632222d-b961-44a1-9174-b51d8fcc0c13', 'fda_g_k_12_r_11_remedial_action', '1', 'Recall', NULL, 1, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (172, 'd70f20c1-0553-457d-9f5c-30c0f10e4e50', 'fda_g_k_12_r_11_remedial_action', '2', 'Repair', NULL, 2, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (173, 'dbd2ce08-d40c-4593-9fdc-e308cfada135', 'fda_g_k_12_r_11_remedial_action', '3', 'Replacement', NULL, 3, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (174, '1ae66098-e401-428f-a6a1-945e570c9f7a', 'fda_g_k_12_r_11_remedial_action', '4', 'Relabeling', NULL, 4, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (175, 'd91c6184-8c73-41e9-843d-b54248124c56', 'fda_g_k_12_r_11_remedial_action', '5', 'Notification', NULL, 5, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (176, 'e82e1b3d-e053-4428-b792-7d341831352f', 'fda_g_k_12_r_11_remedial_action', '6', 'Inspection', NULL, 6, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (177, 'c53b8fe3-090a-41c6-b421-b061d58e54ab', 'fda_g_k_12_r_11_remedial_action', '7', 'Patient Monitoring', NULL, 7, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (178, 'b5a917ed-a8aa-4767-bb8b-8911933ee13e', 'fda_g_k_12_r_11_remedial_action', '8', 'Modification or Adjustment', NULL, 8, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (179, '8c5c9af4-ef8a-4aa7-b09b-44eb9fbabe98', 'fda_g_k_12_r_11_remedial_action', '9', 'Other', NULL, 9, true);
INSERT INTO public.e2b_code_lists (id, audit_id, list_name, code, display_name, description, sort_order, active) VALUES (20, '2188b236-29ce-4f53-806a-a9fd791a42e3', 'age_group', '0', 'Foetus', NULL, 0, true);


--
-- Data for Name: rbac_policy_state; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.rbac_policy_state (singleton, version, updated_at) VALUES (true, 1, '2026-11-27 12:32:27.950027+00');


--
-- Data for Name: ucum_units; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (1, 's', 'second', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (2, 'min', 'minute', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (3, 'h', 'hour', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (4, 'd', 'day', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (5, 'wk', 'week', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (6, 'mo', 'month', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (7, 'a', 'year', NULL, 'time', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (8, 'kg', 'kilogram', NULL, 'mass', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (9, 'g', 'gram', NULL, 'mass', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (10, 'mg', 'milligram', NULL, 'mass', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (11, 'ug', 'microgram', NULL, 'mass', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (12, 'ng', 'nanogram', NULL, 'mass', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (13, 'L', 'liter', NULL, 'volume', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (14, 'mL', 'milliliter', NULL, 'volume', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (15, 'uL', 'microliter', NULL, 'volume', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (16, 'm', 'meter', NULL, 'length', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (17, 'cm', 'centimeter', NULL, 'length', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (18, 'mm', 'millimeter', NULL, 'length', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (19, 'mg/mL', 'milligram per milliliter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (20, 'mg/dL', 'milligram per deciliter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (21, 'mg/L', 'milligram per liter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (22, 'ug/mL', 'microgram per milliliter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (23, 'g/L', 'gram per liter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (24, 'mmol/L', 'millimole per liter', NULL, 'concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (25, 'U/L', 'unit per liter', NULL, 'activity concentration', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (26, 'mg/kg', 'milligram per kilogram', NULL, 'dose', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (27, 'mg/m2', 'milligram per square meter', NULL, 'dose', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (28, '[IU]', 'international unit', NULL, 'dose', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (29, '[IU]/kg', 'international unit per kilogram', NULL, 'dose', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (30, '%', 'percent', NULL, 'ratio', true, '2026-11-27 12:32:28.985568+00');
INSERT INTO public.ucum_units (id, code, display_name, description, unit_type, active, created_at) VALUES (31, '1', 'unity (count)', NULL, 'count', true, '2026-11-27 12:32:28.985568+00');


--
-- Data for Name: user_organization_memberships; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.user_organization_memberships (user_id, organization_id, active, created_at, updated_at, created_by, updated_by) VALUES ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000000', true, '2026-11-27 12:32:27.950027+00', '2026-11-27 12:32:29.800365+00', '00000000-0000-0000-0000-000000000001', NULL);


--
-- Name: e2b_code_lists_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.e2b_code_lists_id_seq', 186, true);


--
-- Name: ucum_units_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.ucum_units_id_seq', 31, true);


--
-- PostgreSQL database dump complete
--
