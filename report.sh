#!/bin/bash
cargo run -- report --source ./out/sample.dacpac --target "host=localhost;database=sample;userid=paul;tlsmode=none;" --profile ./sample/profile.json