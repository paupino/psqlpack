#!/bin/bash
cargo run -- report --source ./out/sample.dacpac --target "Host=localhost;Database=sample;User ID=paul;" --profile ./sample/profile.json