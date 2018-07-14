# Command Line Syntax

The general command line syntax for `psqlpack` follows the convention:

```
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

The project file is what defines the representative database schema. It is currently defined by the following variables:

| Property           | Required   | Type       | Default  | Description |
|--------------------|------------|------------|----------|-------------|
| version            | Yes        | `string`   |          | Must be version 1.0. |
| defaultSchema      | Yes        | `string`   |          | The default schema to be assumed for the database (if none specified). |
| include            | Yes        | `[string]` |          | An array of relative paths to files (or alternatively file patterns to match) to be included in the project definition. |
| extensions         | Yes        | `[string]` |          | An array of relative paths to files (or alternatively file patterns to match) that are PostgreSQL extensions. |
| preDeployScripts   | Yes        | `[string]` |          | An array of relative paths to scripts (or alternatively file patterns to match) set to be used before deployment begins. |
| postDeployScripts  | Yes        | `[string]` |          | An array of relative paths to scripts (or alternatively file patterns to match) set to be used after deployment finishes. |

### Example

```
{
    "version": "1.0",
    "defaultSchema": "public",
    "include": [
        "./**/*.sql"
    ],
    "extensions": [
        "./extensions/postgis.sql"
    ],
    "preDeployScripts": [],
    "postDeployScripts": [
        "./scripts/seed/*.sql"
    ]
}
```

## Publish Profile file format

The publish profile file helps define properties/values that define how we generate psqlpack outputs.

| Property               | Required   | Type                                    | Default  | Description |
|------------------------|------------|-----------------------------------------|----------|-------------|
| version                | Yes        | `string`                                |          | Must be version 1.0. |
| targetConnectionString | No         | [ConnectionString](#connectionstring)   |          | Optional target for publish actions to use. If provided as a command line argument will override this value. |
| generationOptions      | No         | [GenerationOptions](#generationoptions) | Object Defaults | An object specifying various options to configure how publish actions are generated. |
| commandVariables       | No         | [[Pair]](#pair)                         |          | Command variables allow you to specify dynamic variables per script. e.g. for Shard deployments. |

### Pair

| Property | Required   | Type     | Default  | Description |
|----------|------------|----------|----------|-------------|
| name     | Yes        | `string` |          | The name of the pair. |
| value    | Yes        | `string` |          | The value associated with the pair. |

### ConnectionString

| Property | Required   | Type      | Default  | Description |
|----------|------------|-----------|----------|-------------|
| database | Yes        | `string`  |          | The name of the database. |
| server   | Yes        | `string`  |          | The name of the server hosting the database. |
| port     | No         | `integer` | 5432     | The port number of the server. |
| user     | Yes        | `string`  |          | The username to use for authentication. |
| password | Yes        | `string`  |          | The password to use for authentication. |
| tlsMode  | No         | `boolean` | false    | Set to true to use TLS for authentication. |

### GenerationOptions

| Property                | Required   | Type                      | Default         | Description |
|-------------------------|------------|---------------------------|-----------------|-------------|
| alwaysRecreateDatabase  | No         | `boolean`                 | false           | Set to true to always recreate the database. |
| allowUnsafeOperations   | No         | `boolean`                 | false           | Set to true to allow unsafe operations (e.g. enum value mods) to occur. |
| blockOnPossibleDataLoss | No         | `boolean`                 | false           | Set to true to block deployment if data loss is detected. |

### Example

A minimal profile:
```
{
    "version": "1.0"
}
```

A full profile which drops objects such as functions but not tables:
```
{
    "version": "1.0",
    "targetConnectionString": {
        "database": "my_db",
        "server": "localhost",
        "user": "paul",
        "password": "somepassword"
    },
    "generationOptions": {
        "alwaysRecreateDatabase": false,
        "blockOnPossibleDataLoss": true,
        "dropObjectsNotInSource": true,
        "tableChangeMode": {
            "create": true,
            "modify": true,
            "drop": false,
        }
    },
    "commandVariables": [
        { "name": "AUDIT_DB", "value": "audit_db" }
    ]
}
```


## psqlpack package structure

The psqlpack package structure is not the same as the Microsoft equivalent. Fundamentally, it's a zip file which contains the packaged project within `psqlpack` serialized files. These are conveniently configured within folders:

* `extensions`: PostgreSQL extension statements.
* `functions`: All function definitions.
* `schemas`: All schema definitions, including public.
* `scripts`: Any pre/post deployment scripts.
* `tables`: All table definitions.
* `types`: Any custom types defined.
