CREATE TEMP TABLE IF NOT EXISTS idents_data (
  id integer,
  country_id integer,
  state_id integer,
  fq_name character varying(50),
  ident_type ident_type_t,
  CONSTRAINT idents_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.idents(id, country_id, state_id, fq_name, ident_type)
  SELECT d.id, d.country_id, d.state_id, d.fq_name, d.ident_type
  FROM idents_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.idents t WHERE t.id = d.id);

UPDATE data.idents
  SET country_id=d.country_id, state_id=d.state_id, fq_name=d.fq_name, ident_type=d.ident_type
  FROM idents_data d
  WHERE d.id=idents.id;

DELETE FROM data.idents d
  WHERE NOT EXISTS (SELECT 1 FROM idents_data t WHERE t.id = d.id);

DISCARD TEMP;
