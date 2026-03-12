ALTER TABLE study_information
ADD COLUMN IF NOT EXISTS study_type_reaction_kr1 VARCHAR(1)
	CHECK (study_type_reaction_kr1 IN ('1', '2', '3', '4'));
