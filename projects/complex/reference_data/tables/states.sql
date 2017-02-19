CREATE TABLE reference_data.states
(
  id          serial NOT NULL,
  country_id  integer,
  name        character varying(80),
  iso         character varying(10),
  enabled     boolean DEFAULT true,
  gid		  integer,
  CONSTRAINT pk_reference_data_states PRIMARY KEY (id),
  CONSTRAINT fk_reference_data_states__country_id FOREIGN KEY (country_id)
      REFERENCES reference_data.countries (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION,
  CONSTRAINT fk_reference_data_states__gid FOREIGN KEY (gid)
      REFERENCES geo.states (gid) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION      
)