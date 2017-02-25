CREATE TEMP TABLE IF NOT EXISTS constant_versions_data (
  id integer,
  ident_id integer,
  start_date date,
  end_date date,
  CONSTRAINT constant_versions_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.constant_versions(id, ident_id, start_date, end_date)
  SELECT d.id, d.ident_id, d.start_date, d.end_date
  FROM constant_versions_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.constant_versions t WHERE t.id = d.id);

UPDATE data.constant_versions
  SET ident_id=d.ident_id, start_date=d.start_date, end_date=d.end_date
  FROM constant_versions_data d
  WHERE d.id=constant_versions.id;

DELETE FROM data.constant_versions d
  WHERE NOT EXISTS (SELECT 1 FROM constant_versions_data t WHERE t.id = d.id);

DISCARD TEMP;
