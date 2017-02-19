CREATE OR REPLACE FUNCTION reference_data.fn_countries()
RETURNS TABLE (
    name character varying(80),
    iso character varying(2)
)
AS $$
    SELECT countries.name, countries.iso 
    FROM reference_data.countries 
    WHERE countries.enabled=true
    ORDER BY countries.iso
$$
LANGUAGE SQL;