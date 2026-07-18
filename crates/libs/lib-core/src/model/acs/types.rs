/// Resources that can be accessed in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
	Case,
	Patient,
	PatientIdentifier,
	Drug,
	Reaction,
	TestResult,
	Narrative,
	MessageHeader,
	SafetyReport,
	DrugDosage,
	DrugIndication,
	DrugSubstance,
	DrugDeviceCharacteristic,
	DrugReactionAssessment,
	RelatednessAssessment,
	DrugRecurrence,
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
	User,
	Organization,
	AuditLog,
	Settings,
	DashboardNotice,
	EmailNotification,
	Terminology,
	XmlExport,
	XmlImport,
}

/// Actions that can be performed on resources.
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
	Send,
	Lock,
}

/// A permission is a resource and action pair.
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
