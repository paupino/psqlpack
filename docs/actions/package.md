# Package Action

The `package` action builds a psqlpack package (`.psqlpack` file) from an existing database target.

## Example

To package the `example` database project to a file `~/db/example.psqlpack`:
```bash
psqlpack package -s ~/dev/example/example.psqlproj -o ~/db/example.psqlpack
```

## Parameters

| Parameter  | Short | Required   | Type     | Description
|------------|-------|------------|----------| -------------
| --source   | -s    |Yes         | `string` | The path to the source `psqlproj` project file.
| --output   | -o    |Yes         | `string` | The location of the folder to export the psqlpack to
