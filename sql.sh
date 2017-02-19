#!/bin/bash
cargo run -- sql --source ./out/sample.dacpac --target "host=localhost;database=sample;userid=paul;tlsmode=none;" --profile ./sample/publish_profile.json --out ./out/publish.sql