CREATE TEMP TABLE IF NOT EXISTS table_coefficients_data (
  id integer,
  table_version_id integer,
  taxable_amount_less_than numeric(26,6),
  subtraction_amount numeric(26,6),
  multiplier numeric(9,6),
  additional_amount numeric(26,6),
  CONSTRAINT table_coefficients_data_pkey PRIMARY KEY (id)
);

-- No rows found

INSERT INTO data.table_coefficients(id, table_version_id, taxable_amount_less_than, subtraction_amount, multiplier, additional_amount)
  SELECT d.id, d.table_version_id, d.taxable_amount_less_than, d.subtraction_amount, d.multiplier, d.additional_amount
  FROM table_coefficients_data d
  WHERE NOT EXISTS (SELECT 1 FROM data.table_coefficients t WHERE t.id = d.id);

UPDATE data.table_coefficients
  SET table_version_id=d.table_version_id, taxable_amount_less_than=d.taxable_amount_less_than, subtraction_amount=d.subtraction_amount, multiplier=d.multiplier, additional_amount=d.additional_amount
  FROM table_coefficients_data d
  WHERE d.id=table_coefficients.id;

DELETE FROM data.table_coefficients d
  WHERE NOT EXISTS (SELECT 1 FROM table_coefficients_data t WHERE t.id = d.id);

DISCARD TEMP;
