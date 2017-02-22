PSQLPackage
===========
PSQLPackage is intended to be a close PostgreSQL alternative for [Microsoft SQL Server sqlpackage](https://msdn.microsoft.com/en-us/library/hh550080(v=vs.103).aspx). It is intended to support the following tasks:

* Package: Create a DACPAC package (.dacpac file) from a source PostgreSQL project.
* Publish: Incrementally update a database schema to match the schema of a source .dacpac file.  If the database does not exist on the server, the publish operation will create it. Otherwise, an existing database will be updated.
* Report: Generate a JSON report of changes that would be made by a publish action.
* Script: Create an SQL script of the incremental changes that would be applied to the target in order to match the schema of source.

Future tasks may be to:
* Improve the report functionality by providing data movement actions etc.
* Extract functionality to generate a DACPAC from a live PostgreSQL database.
* Export functionality to generate a DACPAC from a live PostgreSQL database with user data included as a post-deployment script.

# Command Line Syntax

```
psqlpackage {action} {options}
```

## Package action

| Parameter  | Required   | Value         | Default  | Description |
|------------|------------|---------------|----------|-------------|
| --source   | Yes        | {string}      |          | The path to the project file that defines our database schema to package. |
| --out      | Yes        | {string}      |          | The path of the DACPAC file (or folder) that should be generated. |
| --type     | No         | [File|Folder] | File     | Useful for debugging; allows you to optionally output a folder representation of the DACPAC. |
| --verbose  | No         | [True|False]  | False    | Outputs lexer output during scan phase. |

## Publish action

| Parameter  | Required   | Value       | Default  | Description |
|------------|------------|-------------|----------|-------------|
| --source   | Yes        | {string}    |          | The path to the source DACPAC file representing the database schema. |
| --target   | No         | [string]    |          | The connection string to the target database to update. Only required if not specified in the publish profile. |
| --profile  | Yes        | {string}    |          | The path to the publish profile defining properties/values to use to help generate outputs. |

## Report action

| Parameter  | Required   | Value       | Default  | Description |
|------------|------------|-------------|----------|-------------|
| --source   | Yes        | {string}    |          | The path to the source DACPAC file representing the database schema. |
| --target   | No         | [string]    |          | The connection string to the target database to update. Only required if not specified in the publish profile. |
| --profile  | Yes        | {string}    |          | The path to the publish profile defining properties/values to use to help generate outputs. |
| --out      | Yes        | {string}    |          | The path to the report file that should be generated. |

## Script action

| Parameter  | Required   | Value       | Default  | Description |
|------------|------------|-------------|----------|-------------|
| --source   | Yes        | {string}    |          | The path to the source DACPAC file representing the database schema. |
| --target   | No         | [string]    |          | The connection string to the target database to update. Only required if not specified in the publish profile. |
| --profile  | Yes        | {string}    |          | The path to the publish profile defining properties/values to use to help generate outputs. |
| --out      | Yes        | {string}    |          | The path to the SQL script that should be generated. |

# File formats

We define a number of custom file formats to help drive the DACPAC process.

## Project file format

The project file is what defines the representative database schema. It is currently defined by the following variables:

| Property           | Type              | Description |
|--------------------|-------------------|-------------|
| version            | {string}          | Must be version 1.0. |
| defaultSchema      | {string}          | The default schema to be assumed for the database (if none specified). |
| preDeployScripts   | {arrayOf(string)} | An array of relative paths to scripts set to be used before deployment begins. |
| postDeployScripts  | {arrayOf(string)} | An array of relative paths to scripts set to be used after deployment finishes. |

## Publish Profile file format

The publish profile file helps define properties/values that define how we generate DACPAC outputs.

| Property               | Type                | Description |
|------------------------|---------------------|-------------|
| version                | {string}            | Must be version 1.0. |
| targetConnectionString | [ConnectionString]  | Optional target for publish actions to use. If provided as a command line argument will override this value. | 
| generationOptions      | {GenerationOptions} | An object specifying various options to configure how publish actions are generated. |
| commandVariables       | {arrayOf(Pair)}     | Command variables allow you to specify dynamic variables per script. e.g. for Shard deployments. |

### Pair

| Property | Type     | Description |
|----------|----------|-------------|
| name     | {string} | The name of the pair. |
| value    | {string} | The value associated with the pair. |

### ConnectionString

| Property | Type     | Description |
|----------|----------|-------------|
| name     | {string} | The name of the pair. |
| value    | {string} | The value associated with the pair. |

### GenerationOptions

| Property                | Type                | Description |
|-------------------------|---------------------|-------------|
| alwaysRecreateDatabase  | {boolean}           | Set to true to always recreate the database. |
| blockOnPossibleDataLoss | {boolean}           | Set to true to block deployment if data loss is detected. |

## DACPAC project structure

The DACPAC project structure is not the same as the Microsoft equivalent. Fundamentally, it's a zip file which contains the parsed project within `psqlpackage` serialized files. These are conveniently configured within folders:

* `extensions`: PostgreSQL extension statements.
* `functions`: All function definitions.
* `schemas`: All schema definitions, including public.
* `scripts`: Any pre/post deployment scripts.
* `tables`: All table definitions.
* `types`: Any custom types defined.