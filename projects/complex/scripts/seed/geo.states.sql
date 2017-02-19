CREATE TEMP TABLE IF NOT EXISTS states_data (
  gid integer,
  statefp character varying(2),
  statens character varying(8),
  affgeoid character varying(11),
  geoid character varying(2),
  stusps character varying(2),
  name character varying(100),
  lsad character varying(2),
  aland double precision,
  awater double precision,
  geom geometry,
  CONSTRAINT states_data_pkey PRIMARY KEY (gid)
);

-- No rows found

INSERT INTO geo.states(gid, statefp, statens, affgeoid, geoid, stusps, name, lsad, aland, awater, geom)
  SELECT d.gid, d.statefp, d.statens, d.affgeoid, d.geoid, d.stusps, d.name, d.lsad, d.aland, d.awater, d.geom
  FROM states_data d
  WHERE NOT EXISTS (SELECT 1 FROM geo.states t WHERE t.gid = d.gid);

UPDATE geo.states
  SET statefp=d.statefp, statens=d.statens, affgeoid=d.affgeoid, geoid=d.geoid, stusps=d.stusps, name=d.name, lsad=d.lsad, aland=d.aland, awater=d.awater, geom=d.geom
  FROM states_data d
  WHERE d.gid=states.gid;

DELETE FROM geo.states d
  WHERE NOT EXISTS (SELECT 1 FROM states_data t WHERE t.gid = d.gid);

DISCARD TEMP;
