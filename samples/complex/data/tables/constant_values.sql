CREATE TABLE data.constant_values (
    id                      serial  NOT NULL,
    constant_version_id     integer NOT NULL,
    value                   numeric(26,6) NOT NULL,
    CONSTRAINT pk_data_constant_values PRIMARY KEY (id),
    CONSTRAINT fk_data_constant_values__constant_version_id FOREIGN KEY (constant_version_id) 
      REFERENCES data.constant_versions (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION
);
