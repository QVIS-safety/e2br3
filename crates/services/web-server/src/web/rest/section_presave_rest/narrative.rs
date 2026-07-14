use super::shared::*;

generate_simple_presave_rest_fns! {
	Bmc: NarrativePresaveBmc,
	Entity: NarrativePresave,
	ForCreate: NarrativePresaveForCreate,
	ForUpdate: NarrativePresaveForUpdate,
	CreateFn: create_narrative_presave,
	ListFn: list_narrative_presaves,
	GetFn: get_narrative_presave,
	UpdateFn: update_narrative_presave,
	DeleteFn: delete_narrative_presave,
	Kind: Narrative
}
