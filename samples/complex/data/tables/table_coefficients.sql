CREATE TABLE data.table_coefficients (
    id                          serial  NOT NULL,
    table_version_id            integer NOT NULL,
    taxable_amount_less_than    numeric(26,6) NULL,
    subtraction_amount          numeric(26,6) DEFAULT 0 NOT NULL,
    multiplier                  numeric (9, 6) NOT NULL,
    additional_amount           numeric(26,6) DEFAULT 0 NOT NULL,
    CONSTRAINT pk_data_table_coefficients PRIMARY KEY (id),
    CONSTRAINT fk_data_table_coefficients__table_version_id FOREIGN KEY (table_version_id) 
      REFERENCES data.table_versions (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION
);
