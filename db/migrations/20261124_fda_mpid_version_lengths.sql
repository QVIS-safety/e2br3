ALTER TABLE drug_information
    ALTER COLUMN mpid_version TYPE VARCHAR(20);

ALTER TABLE past_drug_history
    ALTER COLUMN mpid_version TYPE VARCHAR(20);
