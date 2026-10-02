DROP TRIGGER IF EXISTS audit_fda_device_information ON fda_device_information;
CREATE TRIGGER audit_fda_device_information
    AFTER INSERT OR UPDATE OR DELETE ON fda_device_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

DROP TRIGGER IF EXISTS audit_fda_device_codes ON fda_device_codes;
CREATE TRIGGER audit_fda_device_codes
    AFTER INSERT OR UPDATE OR DELETE ON fda_device_codes
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();
