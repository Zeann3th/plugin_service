ALTER TABLE plugins DROP CONSTRAINT plugins_publisher_id_fkey;
ALTER TABLE users ALTER COLUMN id TYPE BIGINT;
ALTER TABLE plugins ALTER COLUMN publisher_id TYPE BIGINT;
ALTER TABLE plugins ADD CONSTRAINT plugins_publisher_id_fkey
    FOREIGN KEY (publisher_id) REFERENCES users(id) ON DELETE CASCADE;
