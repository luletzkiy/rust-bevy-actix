INSERT INTO testing.coordinates(value, axis)
VALUES ($1, $2)
RETURNING $table_fields;
