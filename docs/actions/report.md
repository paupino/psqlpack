# Report action

The `report` action outputs a report of the incremental changes that need to be made to update the database schema to match the schema of the source `.psqlpack` file or `.psqlproj` project. 

> Note: This command currently only generates raw debug statements and does not provide any meaningful information. The report format is likely to change in the future.

## Example

To generate a report for changes to be made by the `example` database project using the `local` publish profile:
```console
psqlpack report -s ~/dev/example/example.psqlproj -t "host=localhost;userid=paulmason;password=test;database=example" -p ~/dev/example/local.publish -o ~/db/example.report
```

## Parameters

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------|-------------
| --source   | -s    | Yes        | `string` | The source package or project file to use for the deploy report
| --target   | -t    | Yes        | `string` | The connection string of the target database.
| --profile  | -p    | Yes        | `string` | The path to the publish profile to fine tune how the database is published.
| --output   | -o    | Yes        | `string` | The path to the report file that should be generated.