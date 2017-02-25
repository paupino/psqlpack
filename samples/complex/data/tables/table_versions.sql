CREATE TABLE data.table_versions (
    id                  serial   NOT NULL,
    ident_id            integer  NOT NULL,
    start_date          date     NOT NULL,
    end_date            date     NULL,
    CONSTRAINT pk_data_table_versions PRIMARY KEY (id),
    CONSTRAINT fk_data_table_versions__ident_id FOREIGN KEY (ident_id) 
      REFERENCES data.idents (id) MATCH SIMPLE
      ON UPDATE NO ACTION ON DELETE NO ACTION
);