# Script action

The `script` action outputs a SQL file of the incremental changes that need to be made to update the database schema to match the schema of the source `.psqlpack` file or `.psqlproj` project. This is equivalent to the SQL statements and order that is used in the `publish` command.

## Example

To generate the SQL statements for changes to be made by the `example` database project using the `local` publish profile:
```console
psqlpack script -s ~/dev/example/example.psqlproj -t "host=localhost;userid=paulmason;password=test;database=example" -p ~/dev/example/local.publish -o ~/db/example.sql
```

## Parameters

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------|-------------
| --source   | -s    | Yes        | `string` | The path to the source psqlpack or project file representing the database schema. 
| --target   | -t    | Yes        | `string` | The connection string to the target database.
| --profile  | -p    | Yes        | `string` | The path to the publish profile to fine tune how the database is published.
| --output   | -o    | Yes        | `string` | The path to the SQL script that should be generated.

