-- Required regulatory values are validated after draft persistence.
ALTER TABLE drug_information
    ALTER COLUMN drug_characterization DROP NOT NULL,
    ALTER COLUMN medicinal_product DROP NOT NULL;
