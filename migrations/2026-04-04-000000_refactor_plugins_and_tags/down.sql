-- Re-add columns to plugins
ALTER TABLE plugins ADD COLUMN version VARCHAR(50) NOT NULL DEFAULT '0.1.0';
ALTER TABLE plugins ADD COLUMN file_path VARCHAR(500);
ALTER TABLE plugins ADD COLUMN download_count INTEGER DEFAULT 0;
ALTER TABLE plugins ADD COLUMN status plugin_status NOT NULL DEFAULT 'DRAFT';

-- Restore data from latest version
UPDATE plugins p
SET 
    version = pv.version,
    file_path = pv.file_path,
    download_count = pv.download_count,
    status = pv.status
FROM (
    SELECT DISTINCT ON (plugin_id) plugin_id, version, file_path, download_count, status
    FROM plugin_versions
    ORDER BY plugin_id, created_at DESC
) pv
WHERE p.id = pv.plugin_id;

-- Drop new tables
DROP TABLE plugin_versions;
DROP TABLE plugin_tags;
DROP TABLE tags;
