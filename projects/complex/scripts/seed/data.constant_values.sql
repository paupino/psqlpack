CREATE TEMP TABLE IF NOT EXISTS constant_values_data (
  id integer,
  constant_version_id integer,
  value numeric(26,6),
  CONSTRAINT constant_values_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.constant_values(id, constant_version_id, value)
  SELECT d.id, d.constant_version_id, d.value
  FROM constant_values_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.constant_values t WHERE t.id = d.id);

UPDATE data.constant_values
  SET constant_version_id=d.constant_version_id, value=d.value
  FROM constant_values_data d
  WHERE d.id=constant_values.id;

DELETE FROM data.constant_values d
  WHERE NOT EXISTS (SELECT 1 FROM constant_values_data t WHERE t.id = d.id);

DISCARD TEMP;
