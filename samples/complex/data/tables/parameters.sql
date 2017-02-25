CREATE TABLE data.parameters (
    id                  serial   NOT NULL,
    ident_id            integer  NOT NULL,
    start_date          date     NOT NULL,
    end_date            date     NULL,
    name                character varying(50) NOT NULL,
    CONSTRAINT pk_data_parameters PRIMARY KEY (id),
    CONSTRAINT fk_data_parameters__ident_id FOREIGN KEY (ident_id)
      REFERENCES data.idents (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION
);