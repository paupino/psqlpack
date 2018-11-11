## vNext

### Breaking Changes

* Extensions in project file are now in the form `{ "name": "ext" }`. If you are using this construct then you'll need to manually modify the project format. Going forward, Extensions will not be parsed from SQL files (a warning will be generated).

### New

* Extensions are now supported during publish.