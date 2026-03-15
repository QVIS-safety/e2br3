-- DEV ONLY - Brute force recreate the dev database and user, ensuring a clean schema.
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE usename = 'app_user' OR datname = 'app_db';

DROP DATABASE IF EXISTS app_db;
DROP USER IF EXISTS app_user;

CREATE USER app_user PASSWORD 'dev_only_pwd';
ALTER USER app_user CREATEROLE;
CREATE DATABASE app_db OWNER app_user ENCODING 'UTF8' TEMPLATE template0;
GRANT ALL PRIVILEGES ON DATABASE app_db TO app_user;
