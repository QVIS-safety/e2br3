ALTER TABLE study_presaves
    ADD COLUMN IF NOT EXISTS fda_ind_number_occurred VARCHAR(10),
    ADD COLUMN IF NOT EXISTS fda_pre_anda_number_occurred VARCHAR(10);
