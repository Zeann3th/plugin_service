ALTER SEQUENCE plugins_id_seq AS INT;
ALTER TABLE plugins ALTER COLUMN id TYPE INT;
ALTER TABLE plugins ALTER COLUMN id SET DEFAULT nextval('plugins_id_seq');
