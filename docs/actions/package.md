# Package Action

The `package` action builds a psqlpack package (`.psqlpack` file) from an existing database target.

Supported parameters for `package` are:

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------| -------------
| --source   | -s    |Yes         | `string` | The path to the source `psqlproj` project file.
| --output   | -o    |Yes         | `string` | The location of the folder to export the psqlpack to
