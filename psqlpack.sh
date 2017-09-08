#!/bin/bash
case $2 in
    simple)
        db="simple"
        ;;
    complex)
        db="complex"
        ;;
    *)
        echo "Unsupported project: $2 (Try: simple, complex)"
        exit 1
        ;;
esac

username=`whoami`

pushd ./cli > /dev/null
case $1 in
    package)
        echo "Packaging '$db'"
        cargo run -- package --source ../samples/$db/project.json --out ../out/$db.psqlpack --trace
        ;;
    debug-package)
        echo "Packaging '$db'"
        cargo run --features symbols -- package --source ../samples/$db/project.json --out ../out/$db.psqlpack --trace
        ;;        
    publish)
        action="Publishing '$db'"
        cargo run -- publish --source ../out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json --trace
        ;;
    script)
        action="Generating SQL for '$db'"
        cargo run -- script --source ../out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.sql --trace
        ;;
    report)
        action="Generating Report for '$db'"
        cargo run -- report --source ../out/$db.psqlpack --target "host=$db.db;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.json --trace
        ;;
    extract)
        action="Extracting psqlpack for '$db'"
        cargo run -- extract --source "host=$db.db;database=$db;userid=$username;tlsmode=none;" --out ../out/${db}db.psqlpack --trace
        ;;        
    unpack)
        action="Unpacking psqlpack for '$db'"
        unzip ../out/$db.psqlpack -d ../out/$db
        ;;
    *)
        echo "Unsupported command: $1 (Try: package, publish, script, report, extract)"
        exit 1
        ;;
esac
popd > /dev/null
