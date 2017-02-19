CREATE TEMP TABLE IF NOT EXISTS taxes_data (
  id integer,
  ident_id integer,
  name character varying(80),
  tax_classification tax_classification_t,
  start_date date,
  end_date date,
  CONSTRAINT taxes_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.taxes(id, ident_id, name, tax_classification, start_date, end_date)
  SELECT d.id, d.ident_id, d.name, d.tax_classification, d.start_date, d.end_date
  FROM taxes_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.taxes t WHERE t.id = d.id);

UPDATE data.taxes
  SET ident_id=d.ident_id, name=d.name, tax_classification=d.tax_classification, start_date=d.start_date, end_date=d.end_date
  FROM taxes_data d
  WHERE d.id=taxes.id;

DELETE FROM data.taxes d
  WHERE NOT EXISTS (SELECT 1 FROM taxes_data t WHERE t.id = d.id);

DISCARD TEMP;
