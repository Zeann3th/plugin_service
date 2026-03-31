CREATE TYPE plugin_status AS ENUM ('DRAFT', 'PUBLISHED');

ALTER TABLE plugins
    ADD COLUMN status plugin_status NOT NULL DEFAULT 'DRAFT',
    ADD COLUMN file_path VARCHAR(500);
