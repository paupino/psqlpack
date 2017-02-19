CREATE TEMP TABLE IF NOT EXISTS countries_data (
  id integer,
  name character varying(80),
  iso character varying(2),
  enabled boolean,
  CONSTRAINT countries_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO reference_data.countries(id, name, iso, enabled)
  SELECT d.id, d.name, d.iso, d.enabled
  FROM countries_data d
  WHERE NOT EXISTS (SELECT 1 FROM reference_data.countries t WHERE t.id = d.id);

UPDATE reference_data.countries
  SET name=d.name, iso=d.iso, enabled=d.enabled
  FROM countries_data d
  WHERE d.id=countries.id;

DELETE FROM reference_data.countries d
  WHERE NOT EXISTS (SELECT 1 FROM countries_data t WHERE t.id = d.id);

DISCARD TEMP;
