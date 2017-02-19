CREATE TEMP TABLE IF NOT EXISTS table_versions_data (
  id integer,
  ident_id integer,
  start_date date,
  end_date date,
  CONSTRAINT table_versions_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.table_versions(id, ident_id, start_date, end_date)
  SELECT d.id, d.ident_id, d.start_date, d.end_date
  FROM table_versions_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.table_versions t WHERE t.id = d.id);

UPDATE data.table_versions
  SET ident_id=d.ident_id, start_date=d.start_date, end_date=d.end_date
  FROM table_versions_data d
  WHERE d.id=table_versions.id;

DELETE FROM data.table_versions d
  WHERE NOT EXISTS (SELECT 1 FROM table_versions_data t WHERE t.id = d.id);

DISCARD TEMP;
