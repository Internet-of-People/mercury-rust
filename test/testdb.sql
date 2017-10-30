CREATE TABLE IF NOT EXISTS storagetest
(
    id   BIGSERIAL  PRIMARY KEY,
    key  TEXT       UNIQUE NOT NULL,
    data BYTEA      NOT NULL
);

SELECT column_name, data_type, character_maximum_length
FROM INFORMATION_SCHEMA.COLUMNS
WHERE table_name = 'storagetest';