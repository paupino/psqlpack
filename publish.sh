#!/bin/bash
cargo run -- publish --source ./out/sample.dacpac --target "host=localhost;database=sample;userid=paul;tlsmode=none;" --profile ./sample/profile.json