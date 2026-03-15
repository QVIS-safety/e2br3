#[path = "import_persist/c.rs"]
mod c;
#[path = "import_persist/common.rs"]
mod common;
#[path = "import_persist/d.rs"]
mod d;
#[path = "import_persist/e.rs"]
mod e;
#[path = "import_persist/f.rs"]
mod f;
#[path = "import_persist/g.rs"]
mod g;
#[path = "import_persist/h.rs"]
mod h;
/// `tests/import.rs` covers parser-only extraction.
/// `tests/import_persist.rs` covers the full `import_e2b_xml` pipeline and persisted DB state.
///
/// Coverage manifest for the `import_e2b_xml` persistence pipeline:
///
/// - `import_safety_report` -> `c::imports_c_persisted_models`
/// - `import_sender_information` -> `c::imports_c_persisted_models`
/// - `import_primary_sources` -> `c::imports_c_persisted_models`
/// - `import_case_identifiers` -> `c::imports_c_persisted_models`
/// - `import_documents_held_by_sender` -> `c::imports_c_persisted_models`
/// - `import_literature_references` -> `c::imports_c_persisted_models`
/// - `import_study_information` -> `c::imports_c_persisted_models`
/// - `import_receiver_information` -> `c::imports_c_persisted_models`
/// - `import_patient_information` -> `d::imports_d_persisted_models`
/// - `import_patient_identifiers` -> `d::imports_d_persisted_models`
/// - `import_medical_history` -> `d::imports_d_persisted_models`
/// - `import_past_drug_history` -> `d::imports_d_persisted_models`
/// - `import_patient_death` -> `d::imports_d_persisted_models`
/// - `import_parent_information` -> `d::imports_d_persisted_models`
/// - `import_reactions` -> `e::imports_e_persisted_models`
/// - `import_test_results` -> `f::imports_f_persisted_models`
/// - `import_drugs` -> `g::imports_g_persisted_models`
/// - `import_drug_recurrences` -> `g::imports_g_persisted_models`
/// - `import_drug_reaction_assessments` -> `g::imports_g_persisted_models`
/// - `import_narrative` -> `h::imports_h_persisted_models`
#[path = "common/mod.rs"]
mod test_common;
