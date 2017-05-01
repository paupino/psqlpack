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
        cargo run -- package --source ../samples/$db/project.json --out ../out/$db.dacpac
        ;;
    publish)
        action="Publishing '$db'"
        cargo run -- publish --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json
        ;;
    script)
        action="Generating SQL for '$db'"
        cargo run -- script --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.sql
        ;;
    report)
        action="Generating Report for '$db'"
        cargo run -- report --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=$username;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.json
        ;;
    extract)
        action="Extracting DACPAC for '$db'"
        unzip ../out/$db.dacpac -d ../out/$db
        ;;
    *)
        echo "Unsupported command: $1 (Try: package, publish, script, report, extract)"
        exit 1
        ;;
esac
popd > /dev/null