CREATE TABLE reference_data.countries
(
  id        serial NOT NULL,
  name      character varying(80),
  iso       character varying(2),
  enabled   boolean DEFAULT false,
  CONSTRAINT pk_reference_data_countries PRIMARY KEY (id)
)