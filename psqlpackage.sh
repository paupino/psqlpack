#!/bin/bash
case $2 in
    simple)
        db="simple"
        ;;
    complex)
        db="complex"
        ;;
    *)
        echo "Unsupported project: $2"
        exit 1
        ;;
esac

pushd ./cli > /dev/null
case $1 in
    package)
        echo "Packaging '$db'"
        cargo run -- package --source ../samples/$db/project.json --out ../out/$db.dacpac
        ;;
    publish)
        action="Publishing '$db'"
        cargo run -- publish --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=paul;tlsmode=none;" --profile ../samples/$db/publish_profile.json
        ;;
    script)
        action="Generating SQL for '$db'"
        cargo run -- script --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=paul;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.sql
        ;;
    report)
        action="Generating Report for '$db'"
        cargo run -- report --source ../out/$db.dacpac --target "host=localhost;database=$db;userid=paul;tlsmode=none;" --profile ../samples/$db/publish_profile.json --out ../out/$db.json
        ;;
    *)
        echo "Unsupported command: $1"
        exit 1
        ;;
esac
popd > /dev/null