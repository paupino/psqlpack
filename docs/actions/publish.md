# Publish action

The `publish` action incrementally updates a database schema to match the schema of a source `.psqlpack` file or `.psqlproj` project.  If the database does not exist on the server, the publish operation will create it. Otherwise, an existing database will be updated.

## Example

To publish the `example` database project using the `local` publish profile:
```console
psqlpack publish -s ~/dev/example/example.psqlproj -t "host=localhost;userid=paulmason;password=test;database=example" -p ~/dev/example/local.publish 
```

## Parameters

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------|-------------
| --source   | -s    | Yes        | `string` | The path to the source psqlpack or project file representing the database schema. 
| --target   | -t    | Yes        | `string` | The connection string to the target database to update.
| --profile  | -p    | Yes        | `string` | The path to the publish profile to fine tune how the database is published.

