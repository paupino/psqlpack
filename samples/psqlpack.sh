#!/bin/bash
case $2 in
    simple)
        db="simple"
        ;;
    complex)
        db="complex"
        ;;
    trivial)
        db="trivial"
        ;;
    *)
        echo "Unsupported project: $2 (Try: simple, complex)"
        exit 1
        ;;
esac

username=`whoami`

case $1 in
    package)
        echo "Packaging '$db'"
        cargo run -p psqlpack-cli -- package --source samples/$db/$db.psqlproj --output out/$db.psqlpack --trace
        ;;
    debug-package)
        echo "Packaging '$db'"
        cargo run -p psqlpack-cli --features symbols -- package --source samples/$db/$db.psqlproj --output out/$db.psqlpack --trace
        ;;        
    publish)
        action="Publishing '$db'"
        cargo run -p psqlpack-cli -- publish --source out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile samples/$db/local.publish --trace
        ;;
    script)
        action="Generating SQL for '$db'"
        cargo run -p psqlpack-cli -- script --source out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile samples/$db/local.publish --output out/$db.sql --trace
        ;;
    report)
        action="Generating Report for '$db'"
        cargo run -p psqlpack-cli -- report --source out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile samples/$db/local.publish --output out/$db.json --trace
        ;;
    extract)
        action="Extracting psqlpack for '$db'"
        cargo run -p psqlpack-cli -- extract --source "host=$db.db;database=$db;userid=$username;tlsmode=none;" --output out/${db}db.psqlpack --trace
        ;;        
    unpack)
        action="Unpacking psqlpack for '$db'"
        unzip out/$db.psqlpack -d out/$db
        ;;
    *)
        echo "Unsupported command: $1 (Try: package, publish, script, report, extract)"
        exit 1
        ;;
esac