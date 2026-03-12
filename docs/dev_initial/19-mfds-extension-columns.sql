ALTER TABLE primary_sources
ADD COLUMN IF NOT EXISTS qualification_kr1 VARCHAR(1)
	CHECK (qualification_kr1 IN ('1', '2'));

ALTER TABLE relatedness_assessments
ADD COLUMN IF NOT EXISTS result_of_assessment_kr2 VARCHAR(2000);
