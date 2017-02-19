CREATE TEMP TABLE IF NOT EXISTS parameters_data (
  id integer,
  ident_id integer,
  start_date date,
  end_date date,
  name character varying(50),
  CONSTRAINT parameters_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.parameters(id, ident_id, start_date, end_date, name)
  SELECT d.id, d.ident_id, d.start_date, d.end_date, d.name
  FROM parameters_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.parameters t WHERE t.id = d.id);

UPDATE data.parameters
  SET ident_id=d.ident_id, start_date=d.start_date, end_date=d.end_date, name=d.name
  FROM parameters_data d
  WHERE d.id=parameters.id;

DELETE FROM data.parameters d
  WHERE NOT EXISTS (SELECT 1 FROM parameters_data t WHERE t.id = d.id);

DISCARD TEMP;
