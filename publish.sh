#!/bin/bash
cargo run -- publish --source ./out/sample.dacpac --target "host=localhost;database=sample;userid=paul;tlsmode=none;" --profile ./sample/publish_profile.json