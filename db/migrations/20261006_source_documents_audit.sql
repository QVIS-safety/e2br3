DROP TRIGGER IF EXISTS audit_source_documents ON source_documents;
CREATE TRIGGER audit_source_documents
    AFTER INSERT OR UPDATE OR DELETE ON source_documents
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();
