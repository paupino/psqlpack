# Extract action

The `extract` action creates a `.psqlpack` file from the source database.

## Example

To extract the `example` database to a file `~/db/example.psqlpack`:
```console
psqlpack extract -s "host=localhost;userid=paupino;password=test;database=example" -o ~/db/example.psqlpack
```

## Parameters

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------|-------------
| --source   | -s    | Yes        | `string` | The source database connection string.
| --output   | -o    | Yes        | `string` | The file path to output the `.psqlpack` file to.

