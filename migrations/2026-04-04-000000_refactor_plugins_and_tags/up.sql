-- Create tags table
CREATE TABLE tags (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create plugin_tags (many-to-many)
CREATE TABLE plugin_tags (
    plugin_id BIGINT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    tag_id BIGINT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (plugin_id, tag_id)
);

-- Create plugin_versions table
CREATE TABLE plugin_versions (
    id BIGSERIAL PRIMARY KEY,
    plugin_id BIGINT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    file_path VARCHAR(500),
    download_count INTEGER DEFAULT 0,
    status plugin_status NOT NULL DEFAULT 'DRAFT',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (plugin_id, version)
);

-- Migrate existing data from plugins to plugin_versions
INSERT INTO plugin_versions (plugin_id, version, file_path, download_count, status, created_at, updated_at)
SELECT id, version, file_path, COALESCE(download_count, 0), status, created_at, updated_at
FROM plugins;

-- Cleanup plugins table
ALTER TABLE plugins DROP COLUMN version;
ALTER TABLE plugins DROP COLUMN file_path;
ALTER TABLE plugins DROP COLUMN download_count;
ALTER TABLE plugins DROP COLUMN status;
