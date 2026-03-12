//! Permission definitions for the Access Control System
//!
//! Defines resources, actions, and the permission matrix for RBAC.

use crate::ctx::{
	ROLE_ADB_ADMIN, ROLE_ADMIN, ROLE_HEAD_PV, ROLE_MANAGER, ROLE_PVM, ROLE_PVS,
	ROLE_SPONSOR, ROLE_USER, ROLE_VIEWER,
};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

// region:    --- Resource Enum

/// Resources that can be accessed in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
	// Core entities
	Case,
	Patient,
	PatientIdentifier,
	Drug,
	Reaction,
	TestResult,
	Narrative,
	MessageHeader,
	SafetyReport,

	// Nested drug resources
	DrugDosage,
	DrugIndication,
	DrugSubstance,
	DrugDeviceCharacteristic,
	DrugReactionAssessment,
	RelatednessAssessment,
	DrugRecurrence,

	// Other case resources
	CaseIdentifier,
	Receiver,
	PrimarySource,
	SenderInformation,
	LiteratureReference,
	StudyInformation,
	StudyRegistration,
	MedicalHistory,
	PastDrug,
	PatientDeath,
	DeathCause,
	ParentInformation,
	ParentMedicalHistory,
	ParentPastDrug,
	SenderDiagnosis,
	CaseSummary,
	PresaveTemplate,

	// Administrative
	User,
	Organization,
	AuditLog,

	// Terminology
	Terminology,

	// XML operations
	XmlExport,
	XmlImport,
}

// endregion: --- Resource Enum

// region:    --- Action Enum

/// Actions that can be performed on resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
	Create,
	Read,
	Update,
	Delete,
	List,
	Export,
	Import,
	Approve,
}

// endregion: --- Action Enum

// region:    --- Permission Struct

/// A permission is a combination of a Resource and an Action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permission(pub Resource, pub Action);

impl Permission {
	pub const fn new(resource: Resource, action: Action) -> Self {
		Self(resource, action)
	}

	pub fn resource(&self) -> Resource {
		self.0
	}

	pub fn action(&self) -> Action {
		self.1
	}
}

impl std::fmt::Display for Permission {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}.{:?}", self.0, self.1)
	}
}

// endregion: --- Permission Struct

// region:    --- Permission Constants

// Case permissions
pub const CASE_CREATE: Permission = Permission::new(Resource::Case, Action::Create);
pub const CASE_READ: Permission = Permission::new(Resource::Case, Action::Read);
pub const CASE_UPDATE: Permission = Permission::new(Resource::Case, Action::Update);
pub const CASE_DELETE: Permission = Permission::new(Resource::Case, Action::Delete);
pub const CASE_LIST: Permission = Permission::new(Resource::Case, Action::List);
pub const CASE_APPROVE: Permission =
	Permission::new(Resource::Case, Action::Approve);

// Patient permissions
pub const PATIENT_CREATE: Permission =
	Permission::new(Resource::Patient, Action::Create);
pub const PATIENT_READ: Permission =
	Permission::new(Resource::Patient, Action::Read);
pub const PATIENT_UPDATE: Permission =
	Permission::new(Resource::Patient, Action::Update);
pub const PATIENT_DELETE: Permission =
	Permission::new(Resource::Patient, Action::Delete);
pub const PATIENT_LIST: Permission =
	Permission::new(Resource::Patient, Action::List);

// Patient identifiers (D.1.1.x)
pub const PATIENT_IDENTIFIER_CREATE: Permission =
	Permission::new(Resource::PatientIdentifier, Action::Create);
pub const PATIENT_IDENTIFIER_READ: Permission =
	Permission::new(Resource::PatientIdentifier, Action::Read);
pub const PATIENT_IDENTIFIER_UPDATE: Permission =
	Permission::new(Resource::PatientIdentifier, Action::Update);
pub const PATIENT_IDENTIFIER_DELETE: Permission =
	Permission::new(Resource::PatientIdentifier, Action::Delete);
pub const PATIENT_IDENTIFIER_LIST: Permission =
	Permission::new(Resource::PatientIdentifier, Action::List);

// Drug permissions
pub const DRUG_CREATE: Permission = Permission::new(Resource::Drug, Action::Create);
pub const DRUG_READ: Permission = Permission::new(Resource::Drug, Action::Read);
pub const DRUG_UPDATE: Permission = Permission::new(Resource::Drug, Action::Update);
pub const DRUG_DELETE: Permission = Permission::new(Resource::Drug, Action::Delete);
pub const DRUG_LIST: Permission = Permission::new(Resource::Drug, Action::List);

// Drug sub-resources
pub const DRUG_SUBSTANCE_CREATE: Permission =
	Permission::new(Resource::DrugSubstance, Action::Create);
pub const DRUG_SUBSTANCE_READ: Permission =
	Permission::new(Resource::DrugSubstance, Action::Read);
pub const DRUG_SUBSTANCE_UPDATE: Permission =
	Permission::new(Resource::DrugSubstance, Action::Update);
pub const DRUG_SUBSTANCE_DELETE: Permission =
	Permission::new(Resource::DrugSubstance, Action::Delete);
pub const DRUG_SUBSTANCE_LIST: Permission =
	Permission::new(Resource::DrugSubstance, Action::List);

pub const DRUG_DOSAGE_CREATE: Permission =
	Permission::new(Resource::DrugDosage, Action::Create);
pub const DRUG_DOSAGE_READ: Permission =
	Permission::new(Resource::DrugDosage, Action::Read);
pub const DRUG_DOSAGE_UPDATE: Permission =
	Permission::new(Resource::DrugDosage, Action::Update);
pub const DRUG_DOSAGE_DELETE: Permission =
	Permission::new(Resource::DrugDosage, Action::Delete);
pub const DRUG_DOSAGE_LIST: Permission =
	Permission::new(Resource::DrugDosage, Action::List);

pub const DRUG_INDICATION_CREATE: Permission =
	Permission::new(Resource::DrugIndication, Action::Create);
pub const DRUG_INDICATION_READ: Permission =
	Permission::new(Resource::DrugIndication, Action::Read);
pub const DRUG_INDICATION_UPDATE: Permission =
	Permission::new(Resource::DrugIndication, Action::Update);
pub const DRUG_INDICATION_DELETE: Permission =
	Permission::new(Resource::DrugIndication, Action::Delete);
pub const DRUG_INDICATION_LIST: Permission =
	Permission::new(Resource::DrugIndication, Action::List);

pub const DRUG_DEVICE_CHARACTERISTIC_CREATE: Permission =
	Permission::new(Resource::DrugDeviceCharacteristic, Action::Create);
pub const DRUG_DEVICE_CHARACTERISTIC_READ: Permission =
	Permission::new(Resource::DrugDeviceCharacteristic, Action::Read);
pub const DRUG_DEVICE_CHARACTERISTIC_UPDATE: Permission =
	Permission::new(Resource::DrugDeviceCharacteristic, Action::Update);
pub const DRUG_DEVICE_CHARACTERISTIC_DELETE: Permission =
	Permission::new(Resource::DrugDeviceCharacteristic, Action::Delete);
pub const DRUG_DEVICE_CHARACTERISTIC_LIST: Permission =
	Permission::new(Resource::DrugDeviceCharacteristic, Action::List);

pub const DRUG_REACTION_ASSESSMENT_CREATE: Permission =
	Permission::new(Resource::DrugReactionAssessment, Action::Create);
pub const DRUG_REACTION_ASSESSMENT_READ: Permission =
	Permission::new(Resource::DrugReactionAssessment, Action::Read);
pub const DRUG_REACTION_ASSESSMENT_UPDATE: Permission =
	Permission::new(Resource::DrugReactionAssessment, Action::Update);
pub const DRUG_REACTION_ASSESSMENT_DELETE: Permission =
	Permission::new(Resource::DrugReactionAssessment, Action::Delete);
pub const DRUG_REACTION_ASSESSMENT_LIST: Permission =
	Permission::new(Resource::DrugReactionAssessment, Action::List);

pub const RELATEDNESS_ASSESSMENT_CREATE: Permission =
	Permission::new(Resource::RelatednessAssessment, Action::Create);
pub const RELATEDNESS_ASSESSMENT_READ: Permission =
	Permission::new(Resource::RelatednessAssessment, Action::Read);
pub const RELATEDNESS_ASSESSMENT_UPDATE: Permission =
	Permission::new(Resource::RelatednessAssessment, Action::Update);
pub const RELATEDNESS_ASSESSMENT_DELETE: Permission =
	Permission::new(Resource::RelatednessAssessment, Action::Delete);
pub const RELATEDNESS_ASSESSMENT_LIST: Permission =
	Permission::new(Resource::RelatednessAssessment, Action::List);

pub const DRUG_RECURRENCE_CREATE: Permission =
	Permission::new(Resource::DrugRecurrence, Action::Create);
pub const DRUG_RECURRENCE_READ: Permission =
	Permission::new(Resource::DrugRecurrence, Action::Read);
pub const DRUG_RECURRENCE_UPDATE: Permission =
	Permission::new(Resource::DrugRecurrence, Action::Update);
pub const DRUG_RECURRENCE_DELETE: Permission =
	Permission::new(Resource::DrugRecurrence, Action::Delete);
pub const DRUG_RECURRENCE_LIST: Permission =
	Permission::new(Resource::DrugRecurrence, Action::List);

// Reaction permissions
pub const REACTION_CREATE: Permission =
	Permission::new(Resource::Reaction, Action::Create);
pub const REACTION_READ: Permission =
	Permission::new(Resource::Reaction, Action::Read);
pub const REACTION_UPDATE: Permission =
	Permission::new(Resource::Reaction, Action::Update);
pub const REACTION_DELETE: Permission =
	Permission::new(Resource::Reaction, Action::Delete);
pub const REACTION_LIST: Permission =
	Permission::new(Resource::Reaction, Action::List);

// Test Result permissions
pub const TEST_RESULT_CREATE: Permission =
	Permission::new(Resource::TestResult, Action::Create);
pub const TEST_RESULT_READ: Permission =
	Permission::new(Resource::TestResult, Action::Read);
pub const TEST_RESULT_UPDATE: Permission =
	Permission::new(Resource::TestResult, Action::Update);
pub const TEST_RESULT_DELETE: Permission =
	Permission::new(Resource::TestResult, Action::Delete);
pub const TEST_RESULT_LIST: Permission =
	Permission::new(Resource::TestResult, Action::List);

// Narrative permissions
pub const NARRATIVE_CREATE: Permission =
	Permission::new(Resource::Narrative, Action::Create);
pub const NARRATIVE_READ: Permission =
	Permission::new(Resource::Narrative, Action::Read);
pub const NARRATIVE_UPDATE: Permission =
	Permission::new(Resource::Narrative, Action::Update);
pub const NARRATIVE_DELETE: Permission =
	Permission::new(Resource::Narrative, Action::Delete);
pub const NARRATIVE_LIST: Permission =
	Permission::new(Resource::Narrative, Action::List);

// MessageHeader permissions
pub const MESSAGE_HEADER_CREATE: Permission =
	Permission::new(Resource::MessageHeader, Action::Create);
pub const MESSAGE_HEADER_READ: Permission =
	Permission::new(Resource::MessageHeader, Action::Read);
pub const MESSAGE_HEADER_UPDATE: Permission =
	Permission::new(Resource::MessageHeader, Action::Update);
pub const MESSAGE_HEADER_DELETE: Permission =
	Permission::new(Resource::MessageHeader, Action::Delete);
pub const MESSAGE_HEADER_LIST: Permission =
	Permission::new(Resource::MessageHeader, Action::List);

// SafetyReport permissions
pub const SAFETY_REPORT_CREATE: Permission =
	Permission::new(Resource::SafetyReport, Action::Create);
pub const SAFETY_REPORT_READ: Permission =
	Permission::new(Resource::SafetyReport, Action::Read);
pub const SAFETY_REPORT_UPDATE: Permission =
	Permission::new(Resource::SafetyReport, Action::Update);
pub const SAFETY_REPORT_DELETE: Permission =
	Permission::new(Resource::SafetyReport, Action::Delete);
pub const SAFETY_REPORT_LIST: Permission =
	Permission::new(Resource::SafetyReport, Action::List);

// SafetyReport sub-resources
pub const SENDER_INFORMATION_CREATE: Permission =
	Permission::new(Resource::SenderInformation, Action::Create);
pub const SENDER_INFORMATION_READ: Permission =
	Permission::new(Resource::SenderInformation, Action::Read);
pub const SENDER_INFORMATION_UPDATE: Permission =
	Permission::new(Resource::SenderInformation, Action::Update);
pub const SENDER_INFORMATION_DELETE: Permission =
	Permission::new(Resource::SenderInformation, Action::Delete);
pub const SENDER_INFORMATION_LIST: Permission =
	Permission::new(Resource::SenderInformation, Action::List);

pub const PRIMARY_SOURCE_CREATE: Permission =
	Permission::new(Resource::PrimarySource, Action::Create);
pub const PRIMARY_SOURCE_READ: Permission =
	Permission::new(Resource::PrimarySource, Action::Read);
pub const PRIMARY_SOURCE_UPDATE: Permission =
	Permission::new(Resource::PrimarySource, Action::Update);
pub const PRIMARY_SOURCE_DELETE: Permission =
	Permission::new(Resource::PrimarySource, Action::Delete);
pub const PRIMARY_SOURCE_LIST: Permission =
	Permission::new(Resource::PrimarySource, Action::List);

pub const LITERATURE_REFERENCE_CREATE: Permission =
	Permission::new(Resource::LiteratureReference, Action::Create);
pub const LITERATURE_REFERENCE_READ: Permission =
	Permission::new(Resource::LiteratureReference, Action::Read);
pub const LITERATURE_REFERENCE_UPDATE: Permission =
	Permission::new(Resource::LiteratureReference, Action::Update);
pub const LITERATURE_REFERENCE_DELETE: Permission =
	Permission::new(Resource::LiteratureReference, Action::Delete);
pub const LITERATURE_REFERENCE_LIST: Permission =
	Permission::new(Resource::LiteratureReference, Action::List);

pub const STUDY_INFORMATION_CREATE: Permission =
	Permission::new(Resource::StudyInformation, Action::Create);
pub const STUDY_INFORMATION_READ: Permission =
	Permission::new(Resource::StudyInformation, Action::Read);
pub const STUDY_INFORMATION_UPDATE: Permission =
	Permission::new(Resource::StudyInformation, Action::Update);
pub const STUDY_INFORMATION_DELETE: Permission =
	Permission::new(Resource::StudyInformation, Action::Delete);
pub const STUDY_INFORMATION_LIST: Permission =
	Permission::new(Resource::StudyInformation, Action::List);

pub const STUDY_REGISTRATION_CREATE: Permission =
	Permission::new(Resource::StudyRegistration, Action::Create);
pub const STUDY_REGISTRATION_READ: Permission =
	Permission::new(Resource::StudyRegistration, Action::Read);
pub const STUDY_REGISTRATION_UPDATE: Permission =
	Permission::new(Resource::StudyRegistration, Action::Update);
pub const STUDY_REGISTRATION_DELETE: Permission =
	Permission::new(Resource::StudyRegistration, Action::Delete);
pub const STUDY_REGISTRATION_LIST: Permission =
	Permission::new(Resource::StudyRegistration, Action::List);

// Patient sub-resources
pub const MEDICAL_HISTORY_CREATE: Permission =
	Permission::new(Resource::MedicalHistory, Action::Create);
pub const MEDICAL_HISTORY_READ: Permission =
	Permission::new(Resource::MedicalHistory, Action::Read);
pub const MEDICAL_HISTORY_UPDATE: Permission =
	Permission::new(Resource::MedicalHistory, Action::Update);
pub const MEDICAL_HISTORY_DELETE: Permission =
	Permission::new(Resource::MedicalHistory, Action::Delete);
pub const MEDICAL_HISTORY_LIST: Permission =
	Permission::new(Resource::MedicalHistory, Action::List);

pub const PAST_DRUG_CREATE: Permission =
	Permission::new(Resource::PastDrug, Action::Create);
pub const PAST_DRUG_READ: Permission =
	Permission::new(Resource::PastDrug, Action::Read);
pub const PAST_DRUG_UPDATE: Permission =
	Permission::new(Resource::PastDrug, Action::Update);
pub const PAST_DRUG_DELETE: Permission =
	Permission::new(Resource::PastDrug, Action::Delete);
pub const PAST_DRUG_LIST: Permission =
	Permission::new(Resource::PastDrug, Action::List);

pub const PATIENT_DEATH_CREATE: Permission =
	Permission::new(Resource::PatientDeath, Action::Create);
pub const PATIENT_DEATH_READ: Permission =
	Permission::new(Resource::PatientDeath, Action::Read);
pub const PATIENT_DEATH_UPDATE: Permission =
	Permission::new(Resource::PatientDeath, Action::Update);
pub const PATIENT_DEATH_DELETE: Permission =
	Permission::new(Resource::PatientDeath, Action::Delete);
pub const PATIENT_DEATH_LIST: Permission =
	Permission::new(Resource::PatientDeath, Action::List);

pub const DEATH_CAUSE_CREATE: Permission =
	Permission::new(Resource::DeathCause, Action::Create);
pub const DEATH_CAUSE_READ: Permission =
	Permission::new(Resource::DeathCause, Action::Read);
pub const DEATH_CAUSE_UPDATE: Permission =
	Permission::new(Resource::DeathCause, Action::Update);
pub const DEATH_CAUSE_DELETE: Permission =
	Permission::new(Resource::DeathCause, Action::Delete);
pub const DEATH_CAUSE_LIST: Permission =
	Permission::new(Resource::DeathCause, Action::List);

pub const PARENT_INFORMATION_CREATE: Permission =
	Permission::new(Resource::ParentInformation, Action::Create);
pub const PARENT_INFORMATION_READ: Permission =
	Permission::new(Resource::ParentInformation, Action::Read);
pub const PARENT_INFORMATION_UPDATE: Permission =
	Permission::new(Resource::ParentInformation, Action::Update);
pub const PARENT_INFORMATION_DELETE: Permission =
	Permission::new(Resource::ParentInformation, Action::Delete);
pub const PARENT_INFORMATION_LIST: Permission =
	Permission::new(Resource::ParentInformation, Action::List);

pub const PARENT_MEDICAL_HISTORY_CREATE: Permission =
	Permission::new(Resource::ParentMedicalHistory, Action::Create);
pub const PARENT_MEDICAL_HISTORY_READ: Permission =
	Permission::new(Resource::ParentMedicalHistory, Action::Read);
pub const PARENT_MEDICAL_HISTORY_UPDATE: Permission =
	Permission::new(Resource::ParentMedicalHistory, Action::Update);
pub const PARENT_MEDICAL_HISTORY_DELETE: Permission =
	Permission::new(Resource::ParentMedicalHistory, Action::Delete);
pub const PARENT_MEDICAL_HISTORY_LIST: Permission =
	Permission::new(Resource::ParentMedicalHistory, Action::List);

pub const PARENT_PAST_DRUG_CREATE: Permission =
	Permission::new(Resource::ParentPastDrug, Action::Create);
pub const PARENT_PAST_DRUG_READ: Permission =
	Permission::new(Resource::ParentPastDrug, Action::Read);
pub const PARENT_PAST_DRUG_UPDATE: Permission =
	Permission::new(Resource::ParentPastDrug, Action::Update);
pub const PARENT_PAST_DRUG_DELETE: Permission =
	Permission::new(Resource::ParentPastDrug, Action::Delete);
pub const PARENT_PAST_DRUG_LIST: Permission =
	Permission::new(Resource::ParentPastDrug, Action::List);

// Narrative sub-resources
pub const SENDER_DIAGNOSIS_CREATE: Permission =
	Permission::new(Resource::SenderDiagnosis, Action::Create);
pub const SENDER_DIAGNOSIS_READ: Permission =
	Permission::new(Resource::SenderDiagnosis, Action::Read);
pub const SENDER_DIAGNOSIS_UPDATE: Permission =
	Permission::new(Resource::SenderDiagnosis, Action::Update);
pub const SENDER_DIAGNOSIS_DELETE: Permission =
	Permission::new(Resource::SenderDiagnosis, Action::Delete);
pub const SENDER_DIAGNOSIS_LIST: Permission =
	Permission::new(Resource::SenderDiagnosis, Action::List);

pub const CASE_SUMMARY_CREATE: Permission =
	Permission::new(Resource::CaseSummary, Action::Create);
pub const CASE_SUMMARY_READ: Permission =
	Permission::new(Resource::CaseSummary, Action::Read);
pub const CASE_SUMMARY_UPDATE: Permission =
	Permission::new(Resource::CaseSummary, Action::Update);
pub const CASE_SUMMARY_DELETE: Permission =
	Permission::new(Resource::CaseSummary, Action::Delete);
pub const CASE_SUMMARY_LIST: Permission =
	Permission::new(Resource::CaseSummary, Action::List);

// Case identifiers and receiver
pub const CASE_IDENTIFIER_CREATE: Permission =
	Permission::new(Resource::CaseIdentifier, Action::Create);
pub const CASE_IDENTIFIER_READ: Permission =
	Permission::new(Resource::CaseIdentifier, Action::Read);
pub const CASE_IDENTIFIER_UPDATE: Permission =
	Permission::new(Resource::CaseIdentifier, Action::Update);
pub const CASE_IDENTIFIER_DELETE: Permission =
	Permission::new(Resource::CaseIdentifier, Action::Delete);
pub const CASE_IDENTIFIER_LIST: Permission =
	Permission::new(Resource::CaseIdentifier, Action::List);

pub const RECEIVER_CREATE: Permission =
	Permission::new(Resource::Receiver, Action::Create);
pub const RECEIVER_READ: Permission =
	Permission::new(Resource::Receiver, Action::Read);
pub const RECEIVER_UPDATE: Permission =
	Permission::new(Resource::Receiver, Action::Update);
pub const RECEIVER_DELETE: Permission =
	Permission::new(Resource::Receiver, Action::Delete);
pub const RECEIVER_LIST: Permission =
	Permission::new(Resource::Receiver, Action::List);

// Presave template permissions
pub const PRESAVE_TEMPLATE_CREATE: Permission =
	Permission::new(Resource::PresaveTemplate, Action::Create);
pub const PRESAVE_TEMPLATE_READ: Permission =
	Permission::new(Resource::PresaveTemplate, Action::Read);
pub const PRESAVE_TEMPLATE_UPDATE: Permission =
	Permission::new(Resource::PresaveTemplate, Action::Update);
pub const PRESAVE_TEMPLATE_DELETE: Permission =
	Permission::new(Resource::PresaveTemplate, Action::Delete);
pub const PRESAVE_TEMPLATE_LIST: Permission =
	Permission::new(Resource::PresaveTemplate, Action::List);

// User permissions
pub const USER_CREATE: Permission = Permission::new(Resource::User, Action::Create);
pub const USER_READ: Permission = Permission::new(Resource::User, Action::Read);
pub const USER_UPDATE: Permission = Permission::new(Resource::User, Action::Update);
pub const USER_DELETE: Permission = Permission::new(Resource::User, Action::Delete);
pub const USER_LIST: Permission = Permission::new(Resource::User, Action::List);

// Organization permissions
pub const ORG_CREATE: Permission =
	Permission::new(Resource::Organization, Action::Create);
pub const ORG_READ: Permission =
	Permission::new(Resource::Organization, Action::Read);
pub const ORG_UPDATE: Permission =
	Permission::new(Resource::Organization, Action::Update);
pub const ORG_DELETE: Permission =
	Permission::new(Resource::Organization, Action::Delete);
pub const ORG_LIST: Permission =
	Permission::new(Resource::Organization, Action::List);

// AuditLog permissions
pub const AUDIT_READ: Permission = Permission::new(Resource::AuditLog, Action::Read);
pub const AUDIT_LIST: Permission = Permission::new(Resource::AuditLog, Action::List);

// Terminology permissions
pub const TERMINOLOGY_READ: Permission =
	Permission::new(Resource::Terminology, Action::Read);
pub const TERMINOLOGY_IMPORT: Permission =
	Permission::new(Resource::Terminology, Action::Import);
pub const TERMINOLOGY_APPROVE: Permission =
	Permission::new(Resource::Terminology, Action::Approve);

// XML permissions
pub const XML_EXPORT: Permission =
	Permission::new(Resource::XmlExport, Action::Export);
pub const XML_IMPORT: Permission =
	Permission::new(Resource::XmlImport, Action::Import);

// endregion: --- Permission Constants

// region:    --- Role Permission Mappings

/// Returns all permissions for the admin role
fn admin_permissions() -> &'static [Permission] {
	&[
		// Case - full access
		CASE_CREATE,
		CASE_READ,
		CASE_UPDATE,
		CASE_DELETE,
		CASE_LIST,
		CASE_APPROVE,
		// Patient
		PATIENT_CREATE,
		PATIENT_READ,
		PATIENT_UPDATE,
		PATIENT_DELETE,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_CREATE,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_UPDATE,
		PATIENT_IDENTIFIER_DELETE,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_CREATE,
		DRUG_READ,
		DRUG_UPDATE,
		DRUG_DELETE,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_CREATE,
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_UPDATE,
		DRUG_SUBSTANCE_DELETE,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_CREATE,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_UPDATE,
		DRUG_DOSAGE_DELETE,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_CREATE,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_UPDATE,
		DRUG_INDICATION_DELETE,
		DRUG_INDICATION_LIST,
		DRUG_DEVICE_CHARACTERISTIC_CREATE,
		DRUG_DEVICE_CHARACTERISTIC_READ,
		DRUG_DEVICE_CHARACTERISTIC_UPDATE,
		DRUG_DEVICE_CHARACTERISTIC_DELETE,
		DRUG_DEVICE_CHARACTERISTIC_LIST,
		DRUG_REACTION_ASSESSMENT_CREATE,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_UPDATE,
		DRUG_REACTION_ASSESSMENT_DELETE,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_CREATE,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_UPDATE,
		RELATEDNESS_ASSESSMENT_DELETE,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_CREATE,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_UPDATE,
		DRUG_RECURRENCE_DELETE,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_CREATE,
		REACTION_READ,
		REACTION_UPDATE,
		REACTION_DELETE,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_CREATE,
		TEST_RESULT_READ,
		TEST_RESULT_UPDATE,
		TEST_RESULT_DELETE,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_CREATE,
		NARRATIVE_READ,
		NARRATIVE_UPDATE,
		NARRATIVE_DELETE,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_CREATE,
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_UPDATE,
		SENDER_DIAGNOSIS_DELETE,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_CREATE,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_UPDATE,
		CASE_SUMMARY_DELETE,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_CREATE,
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_UPDATE,
		MESSAGE_HEADER_DELETE,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_CREATE,
		SAFETY_REPORT_READ,
		SAFETY_REPORT_UPDATE,
		SAFETY_REPORT_DELETE,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_CREATE,
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_UPDATE,
		SENDER_INFORMATION_DELETE,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_CREATE,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_UPDATE,
		PRIMARY_SOURCE_DELETE,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_CREATE,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_UPDATE,
		LITERATURE_REFERENCE_DELETE,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_CREATE,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_UPDATE,
		STUDY_INFORMATION_DELETE,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_CREATE,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_UPDATE,
		STUDY_REGISTRATION_DELETE,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_CREATE,
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_UPDATE,
		MEDICAL_HISTORY_DELETE,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_CREATE,
		PAST_DRUG_READ,
		PAST_DRUG_UPDATE,
		PAST_DRUG_DELETE,
		PAST_DRUG_LIST,
		PATIENT_DEATH_CREATE,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_UPDATE,
		PATIENT_DEATH_DELETE,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_CREATE,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_UPDATE,
		DEATH_CAUSE_DELETE,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_CREATE,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_UPDATE,
		PARENT_INFORMATION_DELETE,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_CREATE,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_UPDATE,
		PARENT_MEDICAL_HISTORY_DELETE,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_CREATE,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_UPDATE,
		PARENT_PAST_DRUG_DELETE,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_CREATE,
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_UPDATE,
		CASE_IDENTIFIER_DELETE,
		CASE_IDENTIFIER_LIST,
		RECEIVER_CREATE,
		RECEIVER_READ,
		RECEIVER_UPDATE,
		RECEIVER_DELETE,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_CREATE,
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_UPDATE,
		PRESAVE_TEMPLATE_DELETE,
		PRESAVE_TEMPLATE_LIST,
		// User - full access
		USER_CREATE,
		USER_READ,
		USER_UPDATE,
		USER_DELETE,
		USER_LIST,
		// Organization - full access
		ORG_CREATE,
		ORG_READ,
		ORG_UPDATE,
		ORG_DELETE,
		ORG_LIST,
		// AuditLog
		AUDIT_READ,
		AUDIT_LIST,
		// Terminology
		TERMINOLOGY_READ,
		TERMINOLOGY_IMPORT,
		TERMINOLOGY_APPROVE,
		// XML
		XML_EXPORT,
		XML_IMPORT,
	]
}

/// Returns all permissions for the manager role
fn manager_permissions() -> &'static [Permission] {
	&[
		// Case - full access including approve
		CASE_CREATE,
		CASE_READ,
		CASE_UPDATE,
		CASE_DELETE,
		CASE_LIST,
		CASE_APPROVE,
		// Patient
		PATIENT_CREATE,
		PATIENT_READ,
		PATIENT_UPDATE,
		PATIENT_DELETE,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_CREATE,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_UPDATE,
		PATIENT_IDENTIFIER_DELETE,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_CREATE,
		DRUG_READ,
		DRUG_UPDATE,
		DRUG_DELETE,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_CREATE,
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_UPDATE,
		DRUG_SUBSTANCE_DELETE,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_CREATE,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_UPDATE,
		DRUG_DOSAGE_DELETE,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_CREATE,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_UPDATE,
		DRUG_INDICATION_DELETE,
		DRUG_INDICATION_LIST,
		DRUG_DEVICE_CHARACTERISTIC_CREATE,
		DRUG_DEVICE_CHARACTERISTIC_READ,
		DRUG_DEVICE_CHARACTERISTIC_UPDATE,
		DRUG_DEVICE_CHARACTERISTIC_DELETE,
		DRUG_DEVICE_CHARACTERISTIC_LIST,
		DRUG_REACTION_ASSESSMENT_CREATE,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_UPDATE,
		DRUG_REACTION_ASSESSMENT_DELETE,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_CREATE,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_UPDATE,
		RELATEDNESS_ASSESSMENT_DELETE,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_CREATE,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_UPDATE,
		DRUG_RECURRENCE_DELETE,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_CREATE,
		REACTION_READ,
		REACTION_UPDATE,
		REACTION_DELETE,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_CREATE,
		TEST_RESULT_READ,
		TEST_RESULT_UPDATE,
		TEST_RESULT_DELETE,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_CREATE,
		NARRATIVE_READ,
		NARRATIVE_UPDATE,
		NARRATIVE_DELETE,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_CREATE,
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_UPDATE,
		SENDER_DIAGNOSIS_DELETE,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_CREATE,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_UPDATE,
		CASE_SUMMARY_DELETE,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_CREATE,
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_UPDATE,
		MESSAGE_HEADER_DELETE,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_CREATE,
		SAFETY_REPORT_READ,
		SAFETY_REPORT_UPDATE,
		SAFETY_REPORT_DELETE,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_CREATE,
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_UPDATE,
		SENDER_INFORMATION_DELETE,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_CREATE,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_UPDATE,
		PRIMARY_SOURCE_DELETE,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_CREATE,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_UPDATE,
		LITERATURE_REFERENCE_DELETE,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_CREATE,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_UPDATE,
		STUDY_INFORMATION_DELETE,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_CREATE,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_UPDATE,
		STUDY_REGISTRATION_DELETE,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_CREATE,
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_UPDATE,
		MEDICAL_HISTORY_DELETE,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_CREATE,
		PAST_DRUG_READ,
		PAST_DRUG_UPDATE,
		PAST_DRUG_DELETE,
		PAST_DRUG_LIST,
		PATIENT_DEATH_CREATE,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_UPDATE,
		PATIENT_DEATH_DELETE,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_CREATE,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_UPDATE,
		DEATH_CAUSE_DELETE,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_CREATE,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_UPDATE,
		PARENT_INFORMATION_DELETE,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_CREATE,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_UPDATE,
		PARENT_MEDICAL_HISTORY_DELETE,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_CREATE,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_UPDATE,
		PARENT_PAST_DRUG_DELETE,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_CREATE,
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_UPDATE,
		CASE_IDENTIFIER_DELETE,
		CASE_IDENTIFIER_LIST,
		RECEIVER_CREATE,
		RECEIVER_READ,
		RECEIVER_UPDATE,
		RECEIVER_DELETE,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_CREATE,
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_UPDATE,
		PRESAVE_TEMPLATE_DELETE,
		PRESAVE_TEMPLATE_LIST,
		// User - read only
		USER_READ,
		USER_LIST,
		// Organization - read and list
		ORG_READ,
		ORG_LIST,
		// AuditLog - can view
		AUDIT_READ,
		AUDIT_LIST,
		// Terminology
		TERMINOLOGY_READ,
		TERMINOLOGY_IMPORT,
		TERMINOLOGY_APPROVE,
		// XML
		XML_EXPORT,
		XML_IMPORT,
	]
}

/// Returns all permissions for the regular user role
fn user_permissions() -> &'static [Permission] {
	&[
		// Case - CRUD but no delete, no approve
		CASE_CREATE,
		CASE_READ,
		CASE_UPDATE,
		CASE_LIST,
		// Patient
		PATIENT_CREATE,
		PATIENT_READ,
		PATIENT_UPDATE,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_CREATE,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_UPDATE,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_CREATE,
		DRUG_READ,
		DRUG_UPDATE,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_CREATE,
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_UPDATE,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_CREATE,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_UPDATE,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_CREATE,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_UPDATE,
		DRUG_INDICATION_LIST,
		DRUG_DEVICE_CHARACTERISTIC_CREATE,
		DRUG_DEVICE_CHARACTERISTIC_READ,
		DRUG_DEVICE_CHARACTERISTIC_UPDATE,
		DRUG_DEVICE_CHARACTERISTIC_LIST,
		DRUG_REACTION_ASSESSMENT_CREATE,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_UPDATE,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_CREATE,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_UPDATE,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_CREATE,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_UPDATE,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_CREATE,
		REACTION_READ,
		REACTION_UPDATE,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_CREATE,
		TEST_RESULT_READ,
		TEST_RESULT_UPDATE,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_CREATE,
		NARRATIVE_READ,
		NARRATIVE_UPDATE,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_CREATE,
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_UPDATE,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_CREATE,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_UPDATE,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_CREATE,
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_UPDATE,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_CREATE,
		SAFETY_REPORT_READ,
		SAFETY_REPORT_UPDATE,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_CREATE,
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_UPDATE,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_CREATE,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_UPDATE,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_CREATE,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_UPDATE,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_CREATE,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_UPDATE,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_CREATE,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_UPDATE,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_CREATE,
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_UPDATE,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_CREATE,
		PAST_DRUG_READ,
		PAST_DRUG_UPDATE,
		PAST_DRUG_LIST,
		PATIENT_DEATH_CREATE,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_UPDATE,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_CREATE,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_UPDATE,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_CREATE,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_UPDATE,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_CREATE,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_UPDATE,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_CREATE,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_UPDATE,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_CREATE,
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_UPDATE,
		CASE_IDENTIFIER_LIST,
		RECEIVER_CREATE,
		RECEIVER_READ,
		RECEIVER_UPDATE,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_CREATE,
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_UPDATE,
		PRESAVE_TEMPLATE_DELETE,
		PRESAVE_TEMPLATE_LIST,
		// User - read self only (handled at endpoint level)
		USER_READ,
		// Organization - read own
		ORG_READ,
		// Terminology
		TERMINOLOGY_READ,
		// XML - export only
		XML_EXPORT,
	]
}

/// Returns all permissions for the viewer role
fn viewer_permissions() -> &'static [Permission] {
	&[
		// Case - read only
		CASE_READ,
		CASE_LIST,
		// Patient
		PATIENT_READ,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_READ,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_LIST,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_READ,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_READ,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_READ,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_READ,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_READ,
		PAST_DRUG_LIST,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_LIST,
		RECEIVER_READ,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_LIST,
		// User - read only
		USER_READ,
		USER_LIST,
		// Organization - read own
		ORG_READ,
		// XML - export only (viewing)
		XML_EXPORT,
	]
}

// endregion: --- Role Permission Mappings

fn dynamic_roles() -> &'static RwLock<HashMap<String, Vec<Permission>>> {
	static DYNAMIC_ROLES: OnceLock<RwLock<HashMap<String, Vec<Permission>>>> =
		OnceLock::new();
	DYNAMIC_ROLES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn replace_dynamic_roles(map: HashMap<String, Vec<Permission>>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		*guard = map;
	}
}

pub fn upsert_dynamic_role_permissions(role: &str, permissions: Vec<Permission>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.insert(role.trim().to_ascii_lowercase(), permissions);
	}
}

pub fn remove_dynamic_role(role: &str) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.remove(&role.trim().to_ascii_lowercase());
	}
}

pub fn permissions_for_privileges(
	can_view: bool,
	can_review: bool,
	can_lock: bool,
	can_admin: bool,
) -> Vec<Permission> {
	if can_admin {
		return admin_permissions().to_vec();
	}
	if can_review || can_lock {
		return manager_permissions().to_vec();
	}
	if can_view {
		return viewer_permissions().to_vec();
	}
	Vec::new()
}

// region:    --- Permission Checking Functions

/// Returns the permissions for a given role
pub fn role_permissions(role: &str) -> &'static [Permission] {
	match role {
		ROLE_ADMIN => admin_permissions(),
		ROLE_ADB_ADMIN => admin_permissions(),
		ROLE_MANAGER => manager_permissions(),
		ROLE_PVM => manager_permissions(),
		ROLE_HEAD_PV => manager_permissions(),
		ROLE_USER => user_permissions(),
		ROLE_PVS => user_permissions(),
		ROLE_VIEWER => viewer_permissions(),
		ROLE_SPONSOR => viewer_permissions(),
		_ => &[], // Unknown role has no permissions
	}
}

/// Checks if a role has a specific permission
pub fn has_permission(role: &str, permission: Permission) -> bool {
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(perms) = guard.get(&role.trim().to_ascii_lowercase()) {
			return perms.contains(&permission);
		}
	}
	role_permissions(role).contains(&permission)
}

/// Checks if a role has any of the given permissions
pub fn has_any_permission(role: &str, permissions: &[Permission]) -> bool {
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(role_perms) = guard.get(&role.trim().to_ascii_lowercase()) {
			return permissions.iter().any(|p| role_perms.contains(p));
		}
	}
	let role_perms = role_permissions(role);
	permissions.iter().any(|p| role_perms.contains(p))
}

/// Checks if a role has all of the given permissions
pub fn has_all_permissions(role: &str, permissions: &[Permission]) -> bool {
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(role_perms) = guard.get(&role.trim().to_ascii_lowercase()) {
			return permissions.iter().all(|p| role_perms.contains(p));
		}
	}
	let role_perms = role_permissions(role);
	permissions.iter().all(|p| role_perms.contains(p))
}

// endregion: --- Permission Checking Functions

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_admin_has_all_permissions() {
		assert!(has_permission(ROLE_ADMIN, CASE_CREATE));
		assert!(has_permission(ROLE_ADMIN, CASE_DELETE));
		assert!(has_permission(ROLE_ADMIN, USER_CREATE));
		assert!(has_permission(ROLE_ADMIN, USER_DELETE));
		assert!(has_permission(ROLE_ADMIN, ORG_CREATE));
		assert!(has_permission(ROLE_ADMIN, AUDIT_LIST));
	}

	#[test]
	fn test_manager_permissions() {
		// Manager can do case operations
		assert!(has_permission(ROLE_MANAGER, CASE_CREATE));
		assert!(has_permission(ROLE_MANAGER, CASE_DELETE));
		assert!(has_permission(ROLE_MANAGER, CASE_APPROVE));
		// Manager can read users but not create
		assert!(has_permission(ROLE_MANAGER, USER_READ));
		assert!(!has_permission(ROLE_MANAGER, USER_CREATE));
		// Manager cannot manage organizations, but can list/read them
		assert!(!has_permission(ROLE_MANAGER, ORG_CREATE));
		assert!(has_permission(ROLE_MANAGER, ORG_READ));
		assert!(has_permission(ROLE_MANAGER, ORG_LIST));
	}

	#[test]
	fn test_user_permissions() {
		// User can create and update cases
		assert!(has_permission(ROLE_USER, CASE_CREATE));
		assert!(has_permission(ROLE_USER, CASE_UPDATE));
		// User cannot delete cases or approve
		assert!(!has_permission(ROLE_USER, CASE_DELETE));
		assert!(!has_permission(ROLE_USER, CASE_APPROVE));
		// User cannot manage users
		assert!(!has_permission(ROLE_USER, USER_CREATE));
		// User cannot import XML
		assert!(!has_permission(ROLE_USER, XML_IMPORT));
	}

	#[test]
	fn test_viewer_permissions() {
		// Viewer can only read
		assert!(has_permission(ROLE_VIEWER, CASE_READ));
		assert!(has_permission(ROLE_VIEWER, CASE_LIST));
		// Viewer cannot create, update, or delete
		assert!(!has_permission(ROLE_VIEWER, CASE_CREATE));
		assert!(!has_permission(ROLE_VIEWER, CASE_UPDATE));
		assert!(!has_permission(ROLE_VIEWER, CASE_DELETE));
	}

	#[test]
	fn test_unknown_role_has_no_permissions() {
		assert!(!has_permission("unknown", CASE_READ));
		assert!(!has_permission("hacker", USER_DELETE));
	}

	#[test]
	fn test_has_any_permission() {
		assert!(has_any_permission(ROLE_VIEWER, &[CASE_CREATE, CASE_READ]));
		assert!(!has_any_permission(
			ROLE_VIEWER,
			&[CASE_CREATE, CASE_DELETE]
		));
	}

	#[test]
	fn test_has_all_permissions() {
		assert!(has_all_permissions(ROLE_ADMIN, &[CASE_CREATE, CASE_DELETE]));
		assert!(!has_all_permissions(ROLE_VIEWER, &[CASE_READ, CASE_CREATE]));
	}
}

// endregion: --- Tests
