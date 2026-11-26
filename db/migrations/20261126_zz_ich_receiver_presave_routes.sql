ALTER TABLE receiver_presave_routes
    DROP CONSTRAINT receiver_presave_routes_authority_check;
ALTER TABLE receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_authority_check
    CHECK (authority IN ('ich', 'fda', 'mfds'));
