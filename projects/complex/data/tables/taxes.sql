CREATE TABLE data.taxes (
    id                  serial   NOT NULL,
    ident_id            integer  NOT NULL,
    /* TODO: Use a code as id could potentially change? */
    name                character varying(80) NOT NULL,
    tax_classification  tax_classification_t NOT NULL,
    start_date          date     NOT NULL,
    end_date            date     NULL,
    CONSTRAINT pk_data_taxes PRIMARY KEY (id),
    CONSTRAINT fk_data_taxes__ident_id FOREIGN KEY (ident_id)
      REFERENCES data.idents (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION
);
