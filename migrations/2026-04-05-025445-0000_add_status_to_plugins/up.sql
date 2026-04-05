-- Note: plugin_status enum already exists and was renamed in previous migration.
ALTER TABLE plugins ADD COLUMN status plugin_status NOT NULL DEFAULT 'DRAFTED';
