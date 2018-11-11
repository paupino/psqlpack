# Command Line Syntax

The general command line syntax for `psqlpack` follows the convention:

```bash
psqlpack {action} {options}
```

Actions supported are currently:

* [`extract`](actions/extract.md): Builds a psqlpack package (`.psqlpack` file) from an existing database target.
* [`new`](actions/new.md): Generate a starting template for a psqlpack project (`.psqlproj` file) or generate a new publish profile (`.publish` file) defining properties as to how a database schema should be update.
* [`package`](actions/package.md): Create a psqlpack package (`.psqlpack` file) from a source psqlpack project (`.psqlproj`).
* [`publish`](actions/publish.md): Incrementally update a database schema to match the schema of a source `.psqlpack` file or `.psqlproj` project.  If the database does not exist on the server, the publish operation will create it. Otherwise, an existing database will be updated.
* [`report`](actions/report.md): Generate a JSON report of changes that would be made by a publish action.
* [`script`](actions/script.md): Create an SQL script of the incremental changes that would be applied to the target in order to match the schema of source.

All actions support an optional `--trace` argument which turns on verbose level logging.

# File formats

We define a number of custom file formats to help drive the psqlpack process.

## Project file format

The project file is a JSON formatted file which defines how to interpret the files on disk.

| Property            | Required   | Type       | Description 
|---------------------|------------|------------|-------------
| `version`           | Yes        | `string`   | Must be version `1.0`.
| `defaultSchema`     | Yes        | `string`   | The default schema to be assumed for the database (if none specified).
| `preDeployScripts`  | Yes        | `[string]` | An array of relative paths to SQL scripts to be applied before deployment begins.
| `postDeployScripts` | Yes        | `[string]` | An array of relative paths to SQL scripts to be applied after deployment finishes.
| `extensions`        | No         | [`[Extension]`](#extension) | An array of extensions that are required for this project to function. 
| `fileIncludeGlobs`  | No         | `[string]` | An array of globs representing files/folders to be included within your project. Defaults to `["**/*.sql"]`.
| `fileExcludeGlobs`  | No         | `[string]` | An array of globs representing files/folders to be excluded within your project.

### Extension

| Property  | Required   | Type     | Description 
|-----------|------------|----------|-------------
| `name`    | Yes        | `string` | The name of the extension. e.g. `postgis`
| `version` | No         | `string` | The semver of the extension that you'd like installed. If absent, it will use the latest version of what is available on the server.

### Example

```json
{
    "version": "1.0",
    "defaultSchema": "public",
    "preDeployScripts": [],
    "postDeployScripts": [
        "./scripts/seed/*.sql"
    ],
    "extensions": [
        { "name": "postgis", "version": "2.3.7" },
        { "name": "postgis_topology" }
    ]
}
```

## Publish Profile file format

The publish profile file is a JSON formatted file which helps fine tune how the database is published.

| Property            | Required   | Type                                      | Description 
|---------------------|------------|-------------------------------------------|-------------
| `version`           | Yes        | `string`                                  | Must be version `1.0`.
| `generationOptions` | Yes        | [`GenerationOptions`](#generationoptions) | An object specifying various options to configure how publish actions are generated.

### GenerationOptions

| Property                    | Required   | Type                | Description 
|-----------------------------|------------|---------------------|-------------
| `alwaysRecreateDatabase`    | Yes        | `boolean`           | Set to true to always recreate the database.
| `dropEnumValues`            | Yes        | [`Toggle`](#toggle) | Adjust whether enum values can be dropped. No checks are currently performed for usage before dropping so this is considered unsafe.
| `dropTables`                | Yes        | [`Toggle`](#toggle) | Adjust whether tables can be dropped. Data loss could be encountered.
| `dropColumns`               | Yes        | [`Toggle`](#toggle) | Adjust whether columns can be dropped. Data loss could be encountered.
| `dropPrimaryKeyConstraints` | Yes        | [`Toggle`](#toggle) | Adjust whether primary key constraints can be dropped.
| `dropForeignKeyConstraints` | Yes        | [`Toggle`](#toggle) | Adjust whether foreign key constraints can be dropped.
| `dropFunctions`             | Yes        | [`Toggle`](#toggle) | Adjust whether functions can be dropped.
| `dropIndexes`               | Yes        | [`Toggle`](#toggle) | Adjust whether indexes can be dropped.
| `forceConcurrentIndexes`    | Yes        | `boolean`           | Set to true to force all indexes to be applied concurrently.

### Toggle

Toggle allows you to define three options when encountering an action:

* `Error`: An error will be generated and the operation will be halted.
* `Ignore`: The action will not be executed however will not halt the operation.
* `Allow`: The action will be executed.

### Example

```json
{
  "version": "1.0",
  "generationOptions": {
    "alwaysRecreateDatabase": false,
    "dropEnumValues": "Error",
    "dropFunctions": "Error",
    "dropTables": "Error",
    "dropColumns": "Error",
    "dropPrimaryKeyConstraints": "Error",
    "dropForeignKeyConstraints": "Allow"
  }
}
```


## psqlpack package structure

The psqlpack package structure is not the same as the Microsoft equivalent. Fundamentally, it's a zip file which contains the packaged project within `psqlpack` serialized files. These are conveniently configured within folders:

* `extensions`: PostgreSQL extension statements.
* `functions`: All function definitions.
* `indexes`: All index definitions.
* `schemas`: All schema definitions, including public.
* `scripts`: Any pre/post deployment scripts.
* `tables`: All table definitions.
* `types`: Any custom types defined.
