CREATE TEMP TABLE IF NOT EXISTS states_data (
  id integer,
  country_id integer,
  name character varying(80),
  iso character varying(10),
  enabled boolean,
  gid integer,
  CONSTRAINT states_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO reference_data.states(id, country_id, name, iso, enabled, gid)
  SELECT d.id, d.country_id, d.name, d.iso, d.enabled, d.gid
  FROM states_data d
  WHERE NOT EXISTS (SELECT 1 FROM reference_data.states t WHERE t.id = d.id);

UPDATE reference_data.states
  SET country_id=d.country_id, name=d.name, iso=d.iso, enabled=d.enabled, gid=d.gid
  FROM states_data d
  WHERE d.id=states.id;

DELETE FROM reference_data.states d
  WHERE NOT EXISTS (SELECT 1 FROM states_data t WHERE t.id = d.id);

DISCARD TEMP;
