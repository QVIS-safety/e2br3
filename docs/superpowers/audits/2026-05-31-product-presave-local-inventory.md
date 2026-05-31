# Product Presave Local Inventory

## Scope

Product Presave -> DG only. This artifact records current local architecture evidence only. It does not assert reference behavior.

`dgPrdKey` is case-owned and is not a Product Presave field. It appears only in case DG code and Product Presave negative/sentinel tests that prove it is ignored by Product Presave import.

## Commands Run

```bash
mkdir -p /Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/superpowers/audits
```

```bash
cd /Users/hyundonghoon/projects/rust/e2br3
rg -n "export interface ProductPresaveData|productPresaveSchema|register\(|setIfPresent\(parent|mapProductPresave|handleImportProduct|productPresaveDgImportTargets|preserveOnly|importedToDg" \
  frontend/E2BR3-frontend/lib/types/presave.ts \
  frontend/E2BR3-frontend/lib/schemas/presave.ts \
  frontend/E2BR3-frontend/components/presave/ProductForm.tsx \
  frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts \
  frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts \
  frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx \
  frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts
```

```bash
cd /Users/hyundonghoon/projects/rust/e2br3/e2br3
rg -n "ProductPresave|product_presave|mfds_mpid|product_id|preapproval|fda_ind|mfds_device|mfds_regional|substance" \
  crates/libs/lib-core/src/model \
  crates/services/web-server/src/web/rest \
  crates/services/web-server/tests/api \
  crates/libs/lib-core/tests \
  db/bootstrap/01-safetydb-schema.sql
```

```bash
cd /Users/hyundonghoon/projects/rust/e2br3
rg -n "dgPrdKey|dg_prd_key" \
  frontend/E2BR3-frontend/lib/types/presave.ts \
  frontend/E2BR3-frontend/lib/schemas/presave.ts \
  frontend/E2BR3-frontend/lib/presave \
  frontend/E2BR3-frontend/components/presave \
  frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts
```

## Shared Evidence

- Type inventory: `frontend/E2BR3-frontend/lib/types/presave.ts:128`
- Schema inventory: `frontend/E2BR3-frontend/lib/schemas/presave.ts:156`
- ProductForm controls: `frontend/E2BR3-frontend/components/presave/ProductForm.tsx:309`, `:357`, `:369`, `:403`, `:417`, `:430`, `:438`, `:443`, `:451`, `:456`, `:465`, `:474`, `:491`, `:498`, `:508`, `:515`, `:574`, `:580`, `:591`, `:597`, `:605`, `:636`, `:640`, `:645`, `:656`, `:684`, `:695`, `:725`, `:767`, `:800`, `:812`, `:824`, `:835`
- Read mapper: `frontend/E2BR3-frontend/lib/presave/canonicalMappers.ts:281`
- Write mapper parent: `frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts:167`
- Write mapper children: `frontend/E2BR3-frontend/lib/presave/canonicalWriteMappers.ts:210`, `:221`, `:230`, `:239`
- Current DG import: `frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx:341`
- Local contract test categories: `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1363`
- Backend parent model: `crates/libs/lib-core/src/model/presave.rs:807`
- Backend details API: `crates/services/web-server/src/web/rest/section_presave_rest.rs:1748`
- Bootstrap Product Presave tables: `db/bootstrap/01-safetydb-schema.sql:228`, `:273`, `:292`, `:305`, `:319`

## Current Local Fields

| Local Field | Type | ProductForm Control | Schema Key | Read Mapper | Write Mapper | Backend Field/Table | Current Local DG Target | Current Category | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| productId | string | Product ID input | productId | productId/id | product_id | product_presaves.product_id | none; explicitly not drugs.0.productId | preserveOnly | type:128; schema:157; form:357; mapper:283; writer:172; model:814; test:1439 |
| drugCharacterization | string | G.k.1 select | drugCharacterization | drugCharacterization | drug_characterization | product_presaves.drug_characterization | drugs.0.drugCharacterization | importedToDg | type:130; schema:158; form:376; mapper:284; writer:173; SectionG:345; model:815; test:1410 |
| medicinalProduct | string | G.k.2.2 input | medicinalProduct | medicinalProduct | medicinal_product | product_presaves.medicinal_product | drugs.0.medicinalProduct | importedToDg | type:131; schema:159; form:403; mapper:285; writer:174; SectionG:346; model:816; test:1411 |
| medicinalProductNotation | string | Notation textarea | medicinalProductNotation | medicinalProductNotation | medicinal_product_notation | product_presaves.medicinal_product_notation | none | preserveOnly | type:132; schema:160; form:417; mapper:286; writer:175; model:817; test:1440 |
| drugBrandName | string | Brand Name input | drugBrandName | drugBrandName/brandName | brand_name | product_presaves.brand_name | drugs.0.drugBrandName | importedToDg | type:133; schema:161; form:430; mapper:287; writer:176; SectionG:358; model:819; test:1423 |
| drugGenericName | string | Generic Name input | drugGenericName/genericName | generic_name | product_presaves.drug_generic_name | drugs.0.drugGenericName | importedToDg | type:134; schema:162; form:438; mapper:288; writer:177; SectionG:359; model:820; test:1424 |
| obtainDrugCountry | string | G.k.2.4 input | obtainDrugCountry | obtainDrugCountry | obtain_drug_country | product_presaves.obtain_drug_country | drugs.0.obtainDrugCountry | importedToDg | type:135; schema:163; form:767; mapper:289; writer:178; SectionG:354; model:832; test:1419 |
| drugAuthorizationCountry | string | G.k.3.2 input | drugAuthorizationCountry | drugAuthorizationCountry/authorizationCountry | drug_authorization_country | product_presaves.drug_authorization_country | drugs.0.drugAuthorizationCountry | importedToDg | type:136; schema:164; form:812; mapper:290; writer:179; SectionG:355; model:834; test:1420 |
| drugAuthorizationHolder | string | G.k.3.3 input | drugAuthorizationHolder | drugAuthorizationHolder/authorizationHolder | drug_authorization_holder | product_presaves.drug_authorization_holder | drugs.0.drugAuthorizationHolder | importedToDg | type:137; schema:165; form:824; mapper:291; writer:180; SectionG:356; model:835; test:1421 |
| drugAuthorizationNumber | string | G.k.3.1 input | drugAuthorizationNumber | drugAuthorizationNumber/authorizationNumber | drug_authorization_number | product_presaves.drug_authorization_number | drugs.0.drugAuthorizationNumber | importedToDg | type:138; schema:166; form:800; mapper:292; writer:181; SectionG:357; model:833; test:1422 |
| drugBatchNumber | string | Batch Number input | drugBatchNumber | drugBatchNumber/batchNumber | batch_number | no ProductPresave backend column/model field found | drugs.0.drugBatchNumber | importedToDg | type:139; schema:167; form:443; mapper:293; writer:182; SectionG:360; test:1425 |
| manufacturerName | string | no direct ProductForm control | manufacturerName | manufacturerName | manufacturer_name | product_presaves.manufacturer_name | fallback to drugs.0.drugAuthorizationHolder | importedToDg | type:140; schema:168; mapper:294; writer:183; SectionG:356; model:821; test:1432 |
| manufacturerCountry | string | no direct ProductForm control | manufacturerCountry | manufacturerCountry | manufacturer_country | no ProductPresave backend column/model field found | fallback to drugs.0.drugAuthorizationCountry | importedToDg | type:141; schema:169; mapper:295; writer:184; SectionG:355; test:1430 |
| preApprovalIpName | string | Pre-approval IP Name input | preApprovalIpName | preApprovalIpName/preapprovalIpName | preapproval_ip_name | product_presaves.preapproval_ip_name | none | preserveOnly | type:142; schema:170; form:369; mapper:296; writer:185; model:818; test:1441 |
| originalManufacturer | string | Original Manufacturer input | originalManufacturer | originalManufacturer | original_manufacturer | no ProductPresave backend column/model field found | none | preserveOnly | type:143; schema:171; form:451; mapper:297; writer:186; test:1442 |
| sender | string | Sender input/display | sender | sender | sender | no ProductPresave backend column/model field found | none | preserveOnly | type:144; schema:172; form:456; mapper:298; writer:187; test:1438 |
| senderPresaveId | string | Sender select | senderPresaveId | senderPresaveId | sender_presave_id | product_presaves.sender_presave_id | none | preserveOnly | type:145; schema:173; form:309; mapper:299; writer:188; model:813; test:1437 |
| productDescription | string | Description textarea | productDescription | productDescription/description | product_description | product_presaves.product_description | none | preserveOnly | type:146; schema:174; form:465; mapper:300; writer:189; model:822; test:1443 |
| productDeleted | boolean | Deleted checkbox | productDeleted | productDeleted/deleted | deleted | product_presaves.deleted | none | preserveOnly | type:147; schema:175; form:474; mapper:301; writer:171; model:812; test:1444 |
| mpidVersionDateNumber | string | G.k.2.1.1a input | mpidVersionDateNumber | mpidVersionDateNumber/mpidVersion | mpid_version | product_presaves.mpid_version | drugs.0.mpidVersion | importedToDg | type:148; schema:176; form:491; mapper:302; writer:190; SectionG:348; model:824; test:1413 |
| mpid | string | G.k.2.1.1b input | mpid | mpid | mpid | product_presaves.mpid | drugs.0.mpid | importedToDg | type:149; schema:177; form:498; mapper:303; writer:191; SectionG:349; model:823; test:1414 |
| mfdsMpidVersion | string | no direct ProductForm control found | mfdsMpidVersion | mfdsMpidVersion/mfds_mpid_version | mfds_mpid_version | product_presaves.mfds_mpid_version | drugs.0.mfdsMpidVersion | importedToDg | type:150; schema:178; mapper:304; writer:192; SectionG:350; model:826; test:1415 |
| mfdsMpid | string | no direct ProductForm control found | mfdsMpid | mfdsMpid/mfds_mpid | mfds_mpid | product_presaves.mfds_mpid | drugs.0.mfdsMpid | importedToDg | type:151; schema:179; mapper:305; writer:193; SectionG:351; model:825; test:1416 |
| mfdsDeviceInfo | MfdsDeviceInfo | MFDS Medical Device Information controls | mfdsDeviceInfo | mapProductMfdsDeviceInfo(raw) | productMfdsDeviceItems(data) derived rows | product_presave_mfds_device_items, normalized into frontend object | drugs.0.mfdsDeviceInfo | importedToDg | type:152; schema:180; form:656; mapper:306; writer:239; SectionG:394; API:1753; test:1387 |
| mfdsDeviceItems | array | no direct control; storage backing for mfdsDeviceInfo | mfdsDeviceItems | mapProductMfdsDeviceItems(raw) | productMfdsDeviceItems(data) | product_presave_mfds_device_items | no direct target; imported through mfdsDeviceInfo after mapper normalization | preserveOnly | type:153; schema:181; mapper:307; writer:239; API:1753; test:1449 |
| phpidVersionDateNumber | string | G.k.2.1.2a input | phpidVersionDateNumber | phpidVersionDateNumber/phpidVersion | phpid_version | product_presaves.phpid_version | drugs.0.phpidVersion | importedToDg | type:161; schema:189; form:508; mapper:308; writer:194; SectionG:352; model:828; test:1417 |
| phpid | string | G.k.2.1.2b input | phpid | phpid | phpid | product_presaves.phpid | drugs.0.phpid | importedToDg | type:162; schema:190; form:515; mapper:309; writer:195; SectionG:353; model:827; test:1418 |
| mfdsStudyNumber | string | MFDS Study No. input | mfdsStudyNumber | mfdsStudyNumber | mfds_study_number | no ProductPresave backend column/model field found | none | preserveOnly | type:163; schema:191; form:636; mapper:310; writer:196; test:1450 |
| mfdsProtocolNumber | string | MFDS Protocol No. input | mfdsProtocolNumber | mfdsProtocolNumber | mfds_protocol_number | no ProductPresave backend column/model field found | none | preserveOnly | type:164; schema:192; form:640; mapper:311; writer:197; test:1451 |
| mfdsOtherStudiesType | string | Other Studies Type input | mfdsOtherStudiesType | mfdsOtherStudiesType | study_type_reaction_kr1 | no ProductPresave backend column/model field found | none | preserveOnly | type:165; schema:193; form:645; mapper:312; writer:198; test:1452 |
| fdaIndNumberOccurred | string | FDA.C.5.5a input | fdaIndNumberOccurred | fdaIndNumberOccurred | fda_ind_number_occurred | product_presaves.fda_ind_number_occurred | none | preserveOnly | type:166; schema:194; form:684; mapper:313; writer:199; model:836; test:1445 |
| fdaPreAndaNumberOccurred | string | FDA.C.5.5b input | fdaPreAndaNumberOccurred | fdaPreAndaNumberOccurred | fda_pre_anda_number_occurred | product_presaves.fda_pre_anda_number_occurred | none | preserveOnly | type:167; schema:195; form:695; mapper:314; writer:200; model:837; test:1446 |
| fdaCrossReportedIndNumbers | array | FDA.C.5.6.r rows | fdaCrossReportedIndNumbers | childRows fdaCrossReportedIndNumbers/fdaCrossReportedInds | product_presave_fda_cross_reported_inds rows | product_presave_fda_cross_reported_inds | none | preserveOnly | type:168; schema:196; form:725; mapper:315; writer:221; API:1751; test:1447 |
| substances | array | G.k.2.3.r rows | substances | childRows substances/activeSubstances | product_presave_substances rows | product_presave_substances | drugs.0.activeSubstances | importedToDg | type:176; schema:209; form:574; mapper:321; writer:210; SectionG:395; API:1750; test:1386 |
| mfdsRegionalItems | array | no direct ProductForm control found | no schema key found | childRows mfdsRegionalItems | product_presave_mfds_regional_items rows | product_presave_mfds_regional_items | none | preserveOnly | type:189; mapper:331; writer:230; API:1752; test:1448 |
| drugObtainedCountry | string | no direct ProductForm control found | drugObtainedCountry | drugObtainedCountry | drug_obtained_country | no ProductPresave backend column/model field found | fallback to drugs.0.obtainDrugCountry | importedToDg | type:199; schema:222; mapper:337; writer:201; SectionG:354; test:1428 |
| investigationalProductBlinded | boolean | G.k.2.5 checkbox | investigationalProductBlinded | investigationalProductBlinded | investigational_product_blinded | product_presaves.investigational_product_blinded | drugs.0.investigationalProductBlinded | importedToDg | type:200; schema:223; form:774; mapper:338; writer:202; SectionG:347; model:831; test:1412 |
| authorizationNumber | string | no direct ProductForm control found | authorizationNumber | authorizationNumber | authorization_number | no ProductPresave backend column/model field found | fallback to drugs.0.drugAuthorizationNumber | importedToDg | type:201; schema:224; mapper:339; writer:203; SectionG:357; test:1433 |
| authorizationCountry | string | no direct ProductForm control found | authorizationCountry | authorizationCountry | authorization_country | no ProductPresave backend column/model field found | fallback to drugs.0.drugAuthorizationCountry | importedToDg | type:202; schema:225; mapper:340; writer:204; SectionG:355; test:1429 |
| holderApplicantName | string | no direct ProductForm control found | holderApplicantName | holderApplicantName | holder_applicant_name | no ProductPresave backend column/model field found | fallback to drugs.0.drugAuthorizationHolder | importedToDg | type:203; schema:226; mapper:341; writer:205; SectionG:356; test:1431 |
| holderApplicantNameNotation | string | Holder/applicant notation textarea | holderApplicantNameNotation | holderApplicantNameNotation | holder_applicant_name_notation | product_presaves.holder_applicant_name_notation | none | preserveOnly | type:204; schema:227; form:835; mapper:342; writer:206; model:838; test:1453 |

## Local Gaps and Contradictions

- `drugBatchNumber`, `manufacturerCountry`, `originalManufacturer`, `sender`, `mfdsStudyNumber`, `mfdsProtocolNumber`, `mfdsOtherStudiesType`, `drugObtainedCountry`, `authorizationNumber`, `authorizationCountry`, and `holderApplicantName` are present in the frontend type/schema or mapper surface, but no matching Product Presave parent backend column/model field was found in the searched backend files.
- `drugGenericName` writes `generic_name`, while the backend Product Presave evidence uses `drug_generic_name`; this is a local mapper/backend naming mismatch to resolve after reference classification.
- ProductForm `originalManufacturer` labels/audits against `manufacturer_name`, while `manufacturerName` is the field that maps to backend `manufacturer_name`; this is local evidence of current naming ambiguity, not reference evidence.
- `mfdsRegionalItems` is present in `ProductPresaveData`, the read mapper, write mapper, backend details API, and bootstrap table, but no `productPresaveSchema` key or ProductForm control was found.
- `mfdsRegionalItems` frontend row shape includes `itemCode` and `itemText`, and the write mapper emits `item_code` and `item_text`; the backend details DTO/table evidence found only `item_type` and `item_value`.
- `mfdsMpidVersion` and `mfdsMpid` are persisted and imported to DG, but no direct ProductForm controls were found in this pass.
- Current local `preserveOnly` is provisional. It means only "not imported by current local SectionG contract", not "reference-confirmed preserve-only".

## dgPrdKey Ownership Search

Search hits:

| File | Hit | Classification |
|---|---|---|
| `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1122` | Case DG fixture has `dgPrdKey` | case-owned test fixture, not Product Presave ownership |
| `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1436` | explicit no-DG-import reason | Product Presave negative sentinel |
| `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1456` | reason assertion | Product Presave negative sentinel |
| `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1460` | stray `dgPrdKey` input sentinel | proves Product Presave import ignores case-owned key |
| `frontend/E2BR3-frontend/__tests__/ui-binding/field-bindings.test.ts:1582` | reason assertion | Product Presave negative sentinel |

No `dgPrdKey` / `dg_prd_key` hits were found in Product Presave type, schema, mappers, or ProductForm.
