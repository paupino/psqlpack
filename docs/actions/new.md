# New action

The `new` action creates a templated `psqlproj` or publish profile. 

## Example

To create a new project named `tax_calculator` in the folder `~/dev/calc`:
```console
psqlpack new project -n tax_calculator -o ~/dev/calc
```

To create a new project named `calc` in the folder `~/dev/calc`:
```console
psqlpack new project -o ~/dev/calc
```

To create a new publish profile named `production` in the folder `~/dev/calc`:
```console
psqlpack new publishprofile -n production -o ~/dev/calc
```

## Parameters

| Parameter  | Short | Required   | Type           | Description
|------------|-------|------------|----------------|-------------
| <template> |       | Yes        | `TemplateType` | The template type to generate. Currently either `project` or `publishprofile`.
| --name     | -n    | No         | `string`       | The name for the created output (if none specified, the name of the current directory is used).
| --output   | -o    | Yes        | `string`       | The location to place the generated output.



