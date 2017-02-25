/* Separate table as this is only maintained/used during development, but not used in production queries */
CREATE TABLE data.idents (
    id                  serial   NOT NULL,
    country_id          int      NOT NULL,
    state_id            int      NULL,
    fq_name             character varying(50) NOT NULL,
    ident_type          ident_type_t NOT NULL,
    CONSTRAINT pk_data_idents PRIMARY KEY (id),
    CONSTRAINT fk_data_idents__country_id FOREIGN KEY (country_id)
      REFERENCES reference_data.countries (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION,
    CONSTRAINT fk_data_idents__state_id FOREIGN KEY (state_id)
      REFERENCES reference_data.states (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION    
);