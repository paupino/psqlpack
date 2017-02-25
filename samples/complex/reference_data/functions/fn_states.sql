CREATE OR REPLACE FUNCTION reference_data.fn_states(country character varying(2))
RETURNS TABLE (
	name character varying(80),
	iso character varying(10)
)
AS $$
	SELECT states.name, states.iso 
	FROM reference_data.states 
    INNER JOIN reference_data.countries ON countries.id=states.country_id
    WHERE countries.iso = $1 AND countries.enabled=true AND states.enabled=true 
    ORDER BY states.iso
$$
LANGUAGE SQL;