// Drug sub-resources REST endpoints (G.k.2.3.r, G.k.4.r, G.k.6.r)

use lib_core::model;
use lib_core::model::acs::{
	DRUG_DEVICE_CHARACTERISTIC_CREATE, DRUG_DEVICE_CHARACTERISTIC_DELETE,
	DRUG_DEVICE_CHARACTERISTIC_LIST, DRUG_DEVICE_CHARACTERISTIC_READ,
	DRUG_DEVICE_CHARACTERISTIC_UPDATE, DRUG_DOSAGE_CREATE, DRUG_DOSAGE_DELETE,
	DRUG_DOSAGE_LIST, DRUG_DOSAGE_READ, DRUG_DOSAGE_UPDATE, DRUG_INDICATION_CREATE,
	DRUG_INDICATION_DELETE, DRUG_INDICATION_LIST, DRUG_INDICATION_READ,
	DRUG_INDICATION_UPDATE, DRUG_SUBSTANCE_CREATE, DRUG_SUBSTANCE_DELETE,
	DRUG_SUBSTANCE_LIST, DRUG_SUBSTANCE_READ, DRUG_SUBSTANCE_UPDATE,
};
use lib_core::model::drug::{
	DosageInformation, DosageInformationBmc, DosageInformationFilter,
	DosageInformationForCreate, DosageInformationForUpdate, DrugActiveSubstance,
	DrugActiveSubstanceBmc, DrugActiveSubstanceFilter, DrugActiveSubstanceForCreate,
	DrugActiveSubstanceForUpdate, DrugDeviceCharacteristic,
	DrugDeviceCharacteristicBmc, DrugDeviceCharacteristicFilter,
	DrugDeviceCharacteristicForCreate, DrugDeviceCharacteristicForUpdate,
	DrugIndication, DrugIndicationBmc, DrugIndicationFilter,
	DrugIndicationForCreate, DrugIndicationForUpdate,
};
use lib_rest_core::Result;
use uuid::Uuid;

fn ensure_drug_scope(
	path_drug_id: Uuid,
	entity_drug_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if path_drug_id != entity_drug_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

// -- Drug Active Substances (G.k.2.3.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugActiveSubstanceBmc,
	Entity: DrugActiveSubstance,
	ForCreate: DrugActiveSubstanceForCreate,
	ForUpdate: DrugActiveSubstanceForUpdate,
	Filter: DrugActiveSubstanceFilter,
	CreateFn: create_drug_active_substance,
	ListFn: list_drug_active_substances,
	GetFn: get_drug_active_substance,
	UpdateFn: update_drug_active_substance,
	DeleteFn: delete_drug_active_substance,
	RestoreFn: restore_drug_active_substance,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_active_substances",
	PermCreate: DRUG_SUBSTANCE_CREATE,
	PermList: DRUG_SUBSTANCE_LIST,
	PermRead: DRUG_SUBSTANCE_READ,
	PermUpdate: DRUG_SUBSTANCE_UPDATE,
	PermDelete: DRUG_SUBSTANCE_DELETE
}

// -- Dosage Information (G.k.4.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DosageInformationBmc,
	Entity: DosageInformation,
	ForCreate: DosageInformationForCreate,
	ForUpdate: DosageInformationForUpdate,
	Filter: DosageInformationFilter,
	CreateFn: create_dosage_information,
	ListFn: list_dosage_information,
	GetFn: get_dosage_information,
	UpdateFn: update_dosage_information,
	DeleteFn: delete_dosage_information,
	RestoreFn: restore_dosage_information,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "dosage_information",
	PermCreate: DRUG_DOSAGE_CREATE,
	PermList: DRUG_DOSAGE_LIST,
	PermRead: DRUG_DOSAGE_READ,
	PermUpdate: DRUG_DOSAGE_UPDATE,
	PermDelete: DRUG_DOSAGE_DELETE
}

// -- Drug Indications (G.k.6.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugIndicationBmc,
	Entity: DrugIndication,
	ForCreate: DrugIndicationForCreate,
	ForUpdate: DrugIndicationForUpdate,
	Filter: DrugIndicationFilter,
	CreateFn: create_drug_indication,
	ListFn: list_drug_indications,
	GetFn: get_drug_indication,
	UpdateFn: update_drug_indication,
	DeleteFn: delete_drug_indication,
	RestoreFn: restore_drug_indication,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_indications",
	PermCreate: DRUG_INDICATION_CREATE,
	PermList: DRUG_INDICATION_LIST,
	PermRead: DRUG_INDICATION_READ,
	PermUpdate: DRUG_INDICATION_UPDATE,
	PermDelete: DRUG_INDICATION_DELETE
}

// -- Drug Device Characteristics (FDA device authority)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugDeviceCharacteristicBmc,
	Entity: DrugDeviceCharacteristic,
	ForCreate: DrugDeviceCharacteristicForCreate,
	ForUpdate: DrugDeviceCharacteristicForUpdate,
	Filter: DrugDeviceCharacteristicFilter,
	CreateFn: create_drug_device_characteristic,
	ListFn: list_drug_device_characteristics,
	GetFn: get_drug_device_characteristic,
	UpdateFn: update_drug_device_characteristic,
	DeleteFn: delete_drug_device_characteristic,
	RestoreFn: restore_drug_device_characteristic,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_device_characteristics",
	PermCreate: DRUG_DEVICE_CHARACTERISTIC_CREATE,
	PermList: DRUG_DEVICE_CHARACTERISTIC_LIST,
	PermRead: DRUG_DEVICE_CHARACTERISTIC_READ,
	PermUpdate: DRUG_DEVICE_CHARACTERISTIC_UPDATE,
	PermDelete: DRUG_DEVICE_CHARACTERISTIC_DELETE
}
