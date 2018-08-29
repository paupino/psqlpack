# psqlpack &emsp; [![Build Status]][travis]

[Build Status]: https://api.travis-ci.org/paupino/psqlpack.svg?branch=master
[travis]: https://travis-ci.org/paupino/psqlpack
[Supported]: docs/images/supported.svg
[Partial]: docs/images/partially-supported.svg
[NotStarted]: docs/images/not-started.svg

[Documentation](docs/index.md)

Psqlpack is a database development tool that is intended to make working with PostgreSQL databases more productive. It was originally inspired by [Microsoft SQL Server sqlpackage](https://docs.microsoft.com/en-us/sql/tools/sqlpackage?view=sql-server-2017) and currently supports the following tasks:

* [Extract](docs/actions/extract.md): Builds a psqlpack package (`.psqlpack` file) from an existing database target.
* [New](docs/actions/new.md): Generate a starting template for a psqlpack project (`.psqlproj` file) or generate a new publish profile (`.publish` file) defining properties as to how a database schema should be update.
* [Package](docs/actions/package.md): Create a psqlpack package (`.psqlpack` file) from a source psqlpack project (`.psqlproj`).
* [Publish](docs/actions/publish.md): Incrementally update a database schema to match the schema of a source `.psqlpack` file or `.psqlproj` project.  If the database does not exist on the server, the publish operation will create it. Otherwise, an existing database will be updated.
* [Report](docs/actions/report.md): Generate a JSON report of changes that would be made by a publish action.
* [Script](docs/actions/script.md): Create an SQL script of the incremental changes that would be applied to the target in order to match the schema of source.

## Is it ready to be used?

Psqlpack can be currently used depending on the features you need for deployment. This project is under active development, so if something is found to be missing then please raise an issue. The following list is a state of feature development:

### Data Object Support

Feature | Status
--------|--------
Schemas | [![Supported](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-schemas)]
Tables | [![Supported](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-tables)]
Types | [![Supported](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-types)]
Primary and Foreign Keys | [![Supported](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-constraints)]
Functions | [![Partial](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-functions)]
Indexes | [![Partial](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-indexes)]
Views | [![NotStarted](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-views)]
Materialized Views | [![NotStarted](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-materialized-views)]
Security Objects | [![NotStarted](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-security)]
Extensions | [![Partial](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-extensions)]
Triggers | [![NotStarted](issues?q=is%3Aopen+is%3Aissue+label%3Afeature-triggers)]

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

### Contributing

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
