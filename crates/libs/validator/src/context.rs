use crate::VocabularyScope;
use lib_core::ctx::Ctx;
use lib_core::model::case::{Case, CaseBmc};
use lib_core::model::case_identifiers::{LinkedReportNumber, OtherCaseIdentifier};
use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::message_header::{MessageHeader, MessageHeaderBmc};
use lib_core::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, NarrativeInformationBmc,
	SenderDiagnosis,
};
use lib_core::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use lib_core::model::patient::{
	AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
	PatientInformationBmc, ReportedCauseOfDeath,
};
use lib_core::model::reaction::Reaction;
use lib_core::model::safety_report::{
	DocumentsHeldBySender, LiteratureReference, PrimarySource, PrimarySourceBmc,
	PrimarySourceFilter, SafetyReportIdentification, SafetyReportIdentificationBmc,
	SenderInformation, SenderInformationBmc, SenderInformationFilter,
	StudyInformation, StudyRegistrationNumber,
};
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::terminology::{
	ControlledTermBmc, MeddraTermBmc, MeddraTermKey, MfdsProductBmc,
	WhodrugProductBmc,
};
use lib_core::model::test_result::TestResult;
use lib_core::model::{ModelManager, Result};
use modql::filter::{OpValValue, OpValsValue};
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Uuid;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Deserialize)]
struct EmbeddedVocabularySnapshot {
	name: String,
	entries: Vec<EmbeddedVocabularyEntry>,
}

#[derive(Debug, Deserialize)]
struct EmbeddedVocabularyEntry {
	code: String,
	scopes: Vec<VocabularyScope>,
}

type SnapshotCodes = HashMap<(String, VocabularyScope), HashSet<String>>;

fn embedded_snapshot_codes() -> Arc<SnapshotCodes> {
	static CODES: OnceLock<Arc<SnapshotCodes>> = OnceLock::new();
	CODES
		.get_or_init(|| {
			let snapshots = [include_str!(
				"../../../../registry/vocabularies/iso639-2.json"
			)];
			let mut codes = SnapshotCodes::new();
			for source in snapshots {
				let snapshot: EmbeddedVocabularySnapshot =
					serde_json::from_str(source)
						.expect("embedded vocabulary snapshot should parse");
				for entry in snapshot.entries {
					for scope in entry.scopes {
						codes
							.entry((snapshot.name.clone(), scope))
							.or_default()
							.insert(entry.code.clone());
					}
				}
			}
			Arc::new(codes)
		})
		.clone()
}

#[derive(Debug, Clone)]
pub struct VocabularyContext {
	meddra_available: bool,
	meddra_versions: HashSet<String>,
	meddra_terms: HashSet<MeddraTermKey>,
	vocabulary_versions: HashMap<String, HashSet<String>>,
	snapshot_codes: Arc<SnapshotCodes>,
}

impl Default for VocabularyContext {
	fn default() -> Self {
		Self {
			meddra_available: false,
			meddra_versions: HashSet::new(),
			meddra_terms: HashSet::new(),
			vocabulary_versions: HashMap::new(),
			snapshot_codes: embedded_snapshot_codes(),
		}
	}
}

impl VocabularyContext {
	pub(crate) fn meddra_available(&self) -> bool {
		self.meddra_available
	}

	pub(crate) fn contains_meddra_version(&self, version: &str) -> bool {
		self.meddra_versions.contains(version)
	}

	pub(crate) fn contains_meddra_term(&self, version: &str, code: &str) -> bool {
		self.meddra_terms.contains(&MeddraTermKey {
			version: version.to_string(),
			code: code.to_string(),
		})
	}

	pub(crate) fn contains_snapshot_code(
		&self,
		vocabulary: &str,
		scope: VocabularyScope,
		code: &str,
	) -> bool {
		self.snapshot_codes
			.get(&(vocabulary.to_string(), scope))
			.is_some_and(|codes| codes.contains(code))
	}

	pub(crate) fn contains_vocabulary_version(
		&self,
		vocabulary: &str,
		version: &str,
	) -> bool {
		self.vocabulary_versions
			.get(vocabulary)
			.is_some_and(|versions| versions.contains(version))
	}

	#[cfg(test)]
	pub(crate) fn for_active_codes(
		entries: &[(&str, VocabularyScope, &str)],
	) -> Self {
		let mut context = Self::default();
		let codes = Arc::make_mut(&mut context.snapshot_codes);
		for (vocabulary, scope, code) in entries {
			codes
				.entry(((*vocabulary).to_string(), *scope))
				.or_default()
				.insert((*code).to_string());
		}
		context
	}

	#[cfg(test)]
	pub(crate) fn for_active_versions(entries: &[(&str, &str)]) -> Self {
		let mut context = Self::default();
		for (vocabulary, version) in entries {
			context
				.vocabulary_versions
				.entry((*vocabulary).to_string())
				.or_default()
				.insert((*version).to_string());
		}
		context
	}

	#[cfg(test)]
	pub(crate) fn for_meddra(keys: &[(&str, &str)]) -> Self {
		let mut context = Self::default();
		context.meddra_available = true;
		context.meddra_versions = keys
			.iter()
			.map(|(version, _)| (*version).to_string())
			.collect();
		context.meddra_terms = keys
			.iter()
			.map(|(version, code)| MeddraTermKey {
				version: (*version).to_string(),
				code: (*code).to_string(),
			})
			.collect();
		context
	}
}

#[derive(Debug, Clone)]
pub struct ValidationContext {
	pub vocabulary: VocabularyContext,
	pub case: Case,
	pub safety_report: Option<SafetyReportIdentification>,
	pub message_header: Option<MessageHeader>,
	pub sender: Option<SenderInformation>,
	pub patient: Option<PatientInformation>,
	pub narrative: Option<NarrativeInformation>,
	pub sender_diagnoses: Vec<SenderDiagnosis>,
	pub case_summaries: Vec<CaseSummaryInformation>,
	pub medical_history: Vec<MedicalHistoryEpisode>,
	pub past_drugs: Vec<PastDrugHistory>,
	pub death_info: Option<PatientDeathInformation>,
	pub reported_causes_of_death: Vec<ReportedCauseOfDeath>,
	pub autopsy_causes_of_death: Vec<AutopsyCauseOfDeath>,
	pub parents: Vec<ParentInformation>,
	pub parent_medical_history: Vec<ParentMedicalHistory>,
	pub parent_past_drugs: Vec<ParentPastDrugHistory>,
	pub primary_sources: Vec<PrimarySource>,
	pub documents_held_by_sender: Vec<DocumentsHeldBySender>,
	pub literature_references: Vec<LiteratureReference>,
	pub other_case_identifiers: Vec<OtherCaseIdentifier>,
	pub linked_report_numbers: Vec<LinkedReportNumber>,
	pub studies: Vec<StudyInformation>,
	pub study_registrations: Vec<StudyRegistrationNumber>,
	pub reactions: Vec<Reaction>,
	pub tests: Vec<TestResult>,
	pub drugs: Vec<DrugInformation>,
	pub active_substances: Vec<DrugActiveSubstance>,
	pub indications: Vec<DrugIndication>,
	pub dosages: Vec<DosageInformation>,
	pub drug_reaction_assessments: Vec<DrugReactionAssessment>,
	pub relatedness_assessments: Vec<RelatednessAssessment>,
	pub patient_identifiers: Vec<PatientIdentifier>,
}

pub async fn load_base_validation_context(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<ValidationContext> {
	set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;
	let (case, safety_report, message_header, sender, patient, narrative) = tokio::try_join!(
		CaseBmc::get(ctx, mm, case_id),
		get_safety_report_optional(ctx, mm, case_id),
		get_message_header_optional(ctx, mm, case_id),
		get_sender_optional(ctx, mm, case_id),
		get_patient_optional(ctx, mm, case_id),
		get_narrative_optional(ctx, mm, case_id),
	)?;

	let (sender_diagnoses, case_summaries) = tokio::try_join!(
		list_sender_diagnoses(mm, narrative.as_ref()),
		list_case_summaries(mm, narrative.as_ref()),
	)?;

	let (medical_history, past_drugs, death_info, parents) = tokio::try_join!(
		list_medical_history(mm, patient.as_ref()),
		list_past_drugs(mm, patient.as_ref()),
		get_death_info_optional(mm, patient.as_ref()),
		list_parents(mm, patient.as_ref()),
	)?;

	let (reported_causes_of_death, autopsy_causes_of_death) = tokio::try_join!(
		list_reported_causes_of_death(mm, death_info.as_ref()),
		list_autopsy_causes_of_death(mm, death_info.as_ref()),
	)?;

	let (parent_medical_history, parent_past_drugs) = tokio::try_join!(
		list_parent_medical_history(mm, &parents),
		list_parent_past_drugs(mm, &parents),
	)?;

	let (
		primary_sources,
		documents_held_by_sender,
		literature_references,
		other_case_identifiers,
		linked_report_numbers,
		studies,
		reactions,
		tests,
		drugs,
	) = tokio::try_join!(
		list_primary_sources(ctx, mm, case_id),
		list_documents_held_by_sender(mm, case_id),
		list_literature_references(mm, case_id),
		list_other_case_identifiers(mm, case_id),
		list_linked_report_numbers(mm, case_id),
		list_studies(mm, case_id),
		lib_core::model::reaction::ReactionBmc::list_by_case(ctx, mm, case_id),
		lib_core::model::test_result::TestResultBmc::list_by_case(ctx, mm, case_id),
		lib_core::model::drug::DrugInformationBmc::list_by_case(ctx, mm, case_id),
	)?;
	let study_registrations = list_study_registrations(mm, &studies).await?;

	let (active_substances, indications, dosages, drug_reaction_assessments) = tokio::try_join!(
		list_active_substances(mm, &drugs),
		list_indications(mm, &drugs),
		list_dosages(mm, &drugs),
		list_drug_reaction_assessments(mm, &drugs),
	)?;
	let relatedness_assessments =
		list_relatedness_assessments(mm, &drug_reaction_assessments).await?;
	let patient_identifiers = list_patient_identifiers(mm, patient.as_ref()).await?;

	let mut validation_ctx = ValidationContext {
		vocabulary: VocabularyContext::default(),
		case,
		safety_report,
		message_header,
		sender,
		patient,
		narrative,
		sender_diagnoses,
		case_summaries,
		medical_history,
		past_drugs,
		death_info,
		reported_causes_of_death,
		autopsy_causes_of_death,
		parents,
		parent_medical_history,
		parent_past_drugs,
		primary_sources,
		documents_held_by_sender,
		literature_references,
		other_case_identifiers,
		linked_report_numbers,
		studies,
		study_registrations,
		reactions,
		tests,
		drugs,
		active_substances,
		indications,
		dosages,
		drug_reaction_assessments,
		relatedness_assessments,
		patient_identifiers,
	};
	validation_ctx.vocabulary = load_vocabulary_context(mm, &validation_ctx).await?;
	Ok(validation_ctx)
}

async fn load_vocabulary_context(
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
) -> Result<VocabularyContext> {
	let requested_keys = case_meddra_keys(validation_ctx);
	let requested_countries = case_country_codes(validation_ctx);
	let requested_product_codes = case_product_codes(validation_ctx);
	let (
		versions,
		terms,
		iso_countries,
		ich_country_extensions,
		edqm_versions,
		mfds_products,
		whodrug_products,
	) = tokio::try_join!(
		MeddraTermBmc::active_versions(mm),
		MeddraTermBmc::existing_active_keys(mm, &requested_keys),
		ControlledTermBmc::existing_active_codes(
			mm,
			"iso3166",
			"country",
			&requested_countries,
		),
		ControlledTermBmc::existing_active_codes(
			mm,
			"iso3166",
			"ich_country",
			&requested_countries,
		),
		ControlledTermBmc::active_release_versions(mm, "edqm", "en"),
		MfdsProductBmc::existing_active_item_seqs(mm, &requested_product_codes),
		WhodrugProductBmc::existing_active_codes(mm, &requested_product_codes),
	)?;
	let meddra_available = !versions.is_empty();
	let mut snapshot_codes = embedded_snapshot_codes();
	Arc::make_mut(&mut snapshot_codes)
		.entry(("ISO3166".to_string(), VocabularyScope::All))
		.or_default()
		.extend(
			iso_countries
				.into_iter()
				.chain(ich_country_extensions.into_iter()),
		);
	Arc::make_mut(&mut snapshot_codes)
		.entry(("MFDS_PRODUCT".to_string(), VocabularyScope::ItemSeq))
		.or_default()
		.extend(mfds_products);
	Arc::make_mut(&mut snapshot_codes)
		.entry(("WHODrug".to_string(), VocabularyScope::All))
		.or_default()
		.extend(whodrug_products);
	let requested_scoped_codes = case_scoped_terminology_codes(validation_ctx);
	for (scope, codes) in requested_scoped_codes {
		let (dictionary, vocabulary) = match scope {
			VocabularyScope::Time
			| VocabularyScope::Gestation
			| VocabularyScope::Dose
			| VocabularyScope::Frequency => ("ich_constrained_ucum", "ICH-UCUM"),
			VocabularyScope::DoseForm | VocabularyScope::Route => ("edqm", "EDQM"),
			VocabularyScope::All | VocabularyScope::ItemSeq => continue,
		};
		let values: Vec<String> = codes.into_iter().collect();
		let existing = ControlledTermBmc::existing_active_codes(
			mm,
			dictionary,
			vocabulary_scope_name(scope),
			&values,
		)
		.await?;
		Arc::make_mut(&mut snapshot_codes)
			.entry((vocabulary.to_string(), scope))
			.or_default()
			.extend(existing);
	}
	let mut vocabulary_versions = HashMap::new();
	vocabulary_versions.insert("EDQM".to_string(), edqm_versions);
	Ok(VocabularyContext {
		meddra_available,
		meddra_versions: versions.into_iter().collect(),
		meddra_terms: terms.into_iter().collect(),
		vocabulary_versions,
		snapshot_codes,
	})
}

fn case_product_codes(validation_ctx: &ValidationContext) -> Vec<String> {
	validation_ctx
		.past_drugs
		.iter()
		.filter_map(|item| item.mfds_medicinal_product_id.as_deref())
		.chain(
			validation_ctx
				.parent_past_drugs
				.iter()
				.filter_map(|item| item.mfds_medicinal_product_id.as_deref()),
		)
		.chain(
			validation_ctx
				.drugs
				.iter()
				.filter_map(|item| item.mfds_mpid.as_deref()),
		)
		.map(str::trim)
		.filter(|code| !code.is_empty())
		.map(str::to_string)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect()
}

fn vocabulary_scope_name(scope: VocabularyScope) -> &'static str {
	match scope {
		VocabularyScope::All => "all",
		VocabularyScope::Time => "time",
		VocabularyScope::Gestation => "gestation",
		VocabularyScope::Dose => "dose",
		VocabularyScope::Frequency => "frequency",
		VocabularyScope::DoseForm => "dose_form",
		VocabularyScope::Route => "route",
		VocabularyScope::ItemSeq => "item_seq",
	}
}

fn case_scoped_terminology_codes(
	validation_ctx: &ValidationContext,
) -> HashMap<VocabularyScope, HashSet<String>> {
	let mut codes = HashMap::<VocabularyScope, HashSet<String>>::new();
	let mut add = |scope: VocabularyScope, value: Option<&str>| {
		if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
			codes.entry(scope).or_default().insert(value.to_string());
		}
	};

	if let Some(patient) = validation_ctx.patient.as_ref() {
		add(VocabularyScope::Time, patient.age_unit.as_deref());
		add(
			VocabularyScope::Gestation,
			patient.gestation_period_unit.as_deref(),
		);
	}
	for parent in &validation_ctx.parents {
		add(VocabularyScope::Time, parent.parent_age_unit.as_deref());
	}
	for reaction in &validation_ctx.reactions {
		add(VocabularyScope::Time, reaction.duration_unit.as_deref());
	}
	for drug in &validation_ctx.drugs {
		add(
			VocabularyScope::Dose,
			drug.cumulative_dose_first_reaction_unit.as_deref(),
		);
		add(
			VocabularyScope::Gestation,
			drug.gestation_period_exposure_unit.as_deref(),
		);
	}
	for dosage in &validation_ctx.dosages {
		add(VocabularyScope::Dose, dosage.dose_unit.as_deref());
		add(VocabularyScope::Frequency, dosage.frequency_unit.as_deref());
		add(VocabularyScope::Time, dosage.duration_unit.as_deref());
		add(
			VocabularyScope::DoseForm,
			dosage.dose_form_termid.as_deref(),
		);
		add(VocabularyScope::Route, dosage.route_termid.as_deref());
		add(
			VocabularyScope::Route,
			dosage.parent_route_termid.as_deref(),
		);
	}
	for assessment in &validation_ctx.drug_reaction_assessments {
		add(
			VocabularyScope::Time,
			assessment.administration_start_interval_unit.as_deref(),
		);
		add(
			VocabularyScope::Time,
			assessment.last_dose_interval_unit.as_deref(),
		);
	}

	codes
}

fn case_country_codes(validation_ctx: &ValidationContext) -> Vec<String> {
	let mut codes = HashSet::new();
	let mut add = |value: Option<&str>| {
		if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
			codes.insert(value.to_string());
		}
	};
	let mut add_identifier = |value: Option<&str>| {
		if let Some(country) =
			value.and_then(|value| value.split_once('-').map(|v| v.0))
		{
			add(Some(country));
		}
	};

	if let Some(report) = validation_ctx.safety_report.as_ref() {
		add_identifier(report.worldwide_unique_id.as_deref());
	}
	for identifier in &validation_ctx.other_case_identifiers {
		add_identifier(Some(identifier.case_identifier.as_str()));
	}
	for source in &validation_ctx.primary_sources {
		add(source.country_code.as_deref());
	}
	if let Some(sender) = validation_ctx.sender.as_ref() {
		add(sender.country_code.as_deref());
	}
	for registration in &validation_ctx.study_registrations {
		add(registration.country_code.as_deref());
	}
	for reaction in &validation_ctx.reactions {
		add(reaction.country_code.as_deref());
	}
	for drug in &validation_ctx.drugs {
		add(drug.obtain_drug_country.as_deref());
		add(drug.manufacturer_country.as_deref());
	}

	codes.into_iter().collect()
}

fn case_meddra_keys(validation_ctx: &ValidationContext) -> Vec<MeddraTermKey> {
	let mut keys = HashSet::<MeddraTermKey>::new();
	let mut add = |version: Option<&str>, code: Option<&str>| {
		let Some(version) = version.map(str::trim).filter(|value| !value.is_empty())
		else {
			return;
		};
		let Some(code) = code.map(str::trim).filter(|value| !value.is_empty())
		else {
			return;
		};
		keys.insert(MeddraTermKey {
			version: version.to_string(),
			code: code.to_string(),
		});
	};

	for item in &validation_ctx.medical_history {
		add(item.meddra_version.as_deref(), item.meddra_code.as_deref());
	}
	for item in &validation_ctx.past_drugs {
		add(
			item.indication_meddra_version.as_deref(),
			item.indication_meddra_code.as_deref(),
		);
		add(
			item.reaction_meddra_version.as_deref(),
			item.reaction_meddra_code.as_deref(),
		);
	}
	for item in &validation_ctx.reported_causes_of_death {
		add(item.meddra_version.as_deref(), item.meddra_code.as_deref());
	}
	for item in &validation_ctx.autopsy_causes_of_death {
		add(item.meddra_version.as_deref(), item.meddra_code.as_deref());
	}
	for item in &validation_ctx.parent_medical_history {
		add(item.meddra_version.as_deref(), item.meddra_code.as_deref());
	}
	for item in &validation_ctx.parent_past_drugs {
		add(
			item.indication_meddra_version.as_deref(),
			item.indication_meddra_code.as_deref(),
		);
		add(
			item.reaction_meddra_version.as_deref(),
			item.reaction_meddra_code.as_deref(),
		);
	}
	for item in &validation_ctx.reactions {
		add(
			item.reaction_meddra_version.as_deref(),
			item.reaction_meddra_code.as_deref(),
		);
	}
	for item in &validation_ctx.tests {
		add(
			item.test_meddra_version.as_deref(),
			item.test_meddra_code.as_deref(),
		);
	}
	for item in &validation_ctx.indications {
		add(
			item.indication_meddra_version.as_deref(),
			item.indication_meddra_code.as_deref(),
		);
	}
	for item in &validation_ctx.sender_diagnoses {
		add(
			item.diagnosis_meddra_version.as_deref(),
			item.diagnosis_meddra_code.as_deref(),
		);
	}

	keys.into_iter().collect()
}

async fn get_safety_report_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SafetyReportIdentification>> {
	match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_message_header_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<MessageHeader>> {
	match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_patient_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<PatientInformation>> {
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_narrative_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<NarrativeInformation>> {
	match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn list_primary_sources(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<PrimarySource>> {
	let filter = PrimarySourceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case_id))])),
		..Default::default()
	};
	let mut rows = PrimarySourceBmc::list(ctx, mm, Some(vec![filter]), None).await?;
	rows.sort_by_key(|row| row.sequence_number);
	Ok(rows)
}

async fn list_other_case_identifiers(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<OtherCaseIdentifier>> {
	let sql = "SELECT * FROM other_case_identifiers WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, OtherCaseIdentifier>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_linked_report_numbers(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<LinkedReportNumber>> {
	let sql = "SELECT * FROM linked_report_numbers WHERE case_id = $1 AND deleted = false ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, LinkedReportNumber>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_documents_held_by_sender(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DocumentsHeldBySender>> {
	let sql =
		"SELECT * FROM documents_held_by_sender WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DocumentsHeldBySender>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_literature_references(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<LiteratureReference>> {
	let sql = "SELECT * FROM literature_references WHERE case_id = $1 AND deleted = false ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, LiteratureReference>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_active_substances(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugActiveSubstance>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugActiveSubstance>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_dosages(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DosageInformation>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM dosage_information WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DosageInformation>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_indications(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugIndication>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_indications WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugIndication>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_drug_reaction_assessments(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugReactionAssessment>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_reaction_assessments WHERE drug_id = ANY($1) ORDER BY drug_id, reaction_id";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugReactionAssessment>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_relatedness_assessments(
	mm: &ModelManager,
	assessments: &[DrugReactionAssessment],
) -> Result<Vec<RelatednessAssessment>> {
	if assessments.is_empty() {
		return Ok(Vec::new());
	}
	let assessment_ids: Vec<Uuid> =
		assessments.iter().map(|assessment| assessment.id).collect();
	let sql = "SELECT * FROM relatedness_assessments WHERE drug_reaction_assessment_id = ANY($1) ORDER BY drug_reaction_assessment_id, sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, RelatednessAssessment>(sql).bind(&assessment_ids),
		)
		.await
		.map_err(Into::into)
}

async fn list_patient_identifiers(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<PatientIdentifier>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM patient_identifiers WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PatientIdentifier>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_medical_history(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<MedicalHistoryEpisode>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM medical_history_episodes WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, MedicalHistoryEpisode>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_sender_diagnoses(
	mm: &ModelManager,
	narrative: Option<&NarrativeInformation>,
) -> Result<Vec<SenderDiagnosis>> {
	let Some(narrative) = narrative else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM sender_diagnoses WHERE narrative_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, SenderDiagnosis>(sql).bind(narrative.id))
		.await
		.map_err(Into::into)
}

async fn list_case_summaries(
	mm: &ModelManager,
	narrative: Option<&NarrativeInformation>,
) -> Result<Vec<CaseSummaryInformation>> {
	let Some(narrative) = narrative else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM case_summary_information WHERE narrative_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, CaseSummaryInformation>(sql).bind(narrative.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_past_drugs(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<PastDrugHistory>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM past_drug_history WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PastDrugHistory>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn get_death_info_optional(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Option<PatientDeathInformation>> {
	let Some(patient) = patient else {
		return Ok(None);
	};
	let sql = "SELECT * FROM patient_death_information WHERE patient_id = $1";
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, PatientDeathInformation>(sql).bind(patient.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_reported_causes_of_death(
	mm: &ModelManager,
	death_info: Option<&PatientDeathInformation>,
) -> Result<Vec<ReportedCauseOfDeath>> {
	let Some(death_info) = death_info else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM reported_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, ReportedCauseOfDeath>(sql).bind(death_info.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_autopsy_causes_of_death(
	mm: &ModelManager,
	death_info: Option<&PatientDeathInformation>,
) -> Result<Vec<AutopsyCauseOfDeath>> {
	let Some(death_info) = death_info else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM autopsy_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, AutopsyCauseOfDeath>(sql).bind(death_info.id))
		.await
		.map_err(Into::into)
}

async fn list_parents(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<ParentInformation>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM parent_information WHERE patient_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentInformation>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_parent_medical_history(
	mm: &ModelManager,
	parents: &[ParentInformation],
) -> Result<Vec<ParentMedicalHistory>> {
	if parents.is_empty() {
		return Ok(Vec::new());
	}
	let parent_ids: Vec<Uuid> = parents.iter().map(|parent| parent.id).collect();
	let sql = "SELECT * FROM parent_medical_history WHERE parent_id = ANY($1) ORDER BY parent_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentMedicalHistory>(sql).bind(parent_ids))
		.await
		.map_err(Into::into)
}

async fn list_parent_past_drugs(
	mm: &ModelManager,
	parents: &[ParentInformation],
) -> Result<Vec<ParentPastDrugHistory>> {
	if parents.is_empty() {
		return Ok(Vec::new());
	}
	let parent_ids: Vec<Uuid> = parents.iter().map(|parent| parent.id).collect();
	let sql = "SELECT * FROM parent_past_drug_history WHERE parent_id = ANY($1) ORDER BY parent_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentPastDrugHistory>(sql).bind(parent_ids))
		.await
		.map_err(Into::into)
}

async fn list_studies(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<StudyInformation>> {
	let sql =
		"SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_study_registrations(
	mm: &ModelManager,
	studies: &[StudyInformation],
) -> Result<Vec<StudyRegistrationNumber>> {
	if studies.is_empty() {
		return Ok(Vec::new());
	}
	let study_ids = studies.iter().map(|study| study.id).collect::<Vec<_>>();
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = ANY($1) AND deleted = false ORDER BY study_information_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyRegistrationNumber>(sql).bind(study_ids))
		.await
		.map_err(Into::into)
}

async fn get_sender_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SenderInformation>> {
	let mut senders = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case_id))])),
		}]),
		None,
	)
	.await?;
	senders.sort_by_key(|sender| sender.created_at);
	Ok(senders.into_iter().next())
}
