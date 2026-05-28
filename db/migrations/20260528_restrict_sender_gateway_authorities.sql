DELETE FROM sender_presave_gateways
WHERE lower(gateway_authority) NOT IN ('fda', 'mfds');

ALTER TABLE sender_presave_gateways
    DROP CONSTRAINT IF EXISTS sender_presave_gateways_authority_valid,
    DROP COLUMN IF EXISTS ema_sender_identifier;

ALTER TABLE sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_authority_valid
    CHECK (gateway_authority IN ('fda', 'mfds'));
