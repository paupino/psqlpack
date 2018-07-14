#!/bin/bash
set -e

version=$1
if [[ ! $version =~ ^\d+\.\d+$ ]]; then
	echo "Please enter a valid version: '$version'"
	exit 1
fi

# This is the array of targets to deploy
targets=(
    "x86_64-apple-darwin"
)

# Build each target first
mkdir -p target/packages/
for target in ${targets[*]}
do
    # Create the binary first
    echo "Compiling $target"
    cargo build --target=$target --release
    cp -f target/$target/release/psqlpack target/packages/
    pushd target/packages/ > /dev/null
    tar -cvzf psqlpack-$target.tar.gz psqlpack
    popd > /dev/null
done

# First, delete the latest release 
to_delete=$(curl -s \
    -H "Accept: application/vnd.github.v3+json" \
    -H "Authorization: token $GITHUB_TOKEN" \
    "https://api.github.com/repos/paupino/psqlpack/releases/tags/$version" | jq -r '.url')
curl -X DELETE -s \
    -H "Accept: application/vnd.github.v3+json" \
    -H "Authorization: token $GITHUB_TOKEN" \
    $to_delete

# Now we create the release in Github
# POST /repos/:owner/:repo/releases
body="{ \
  \"tag_name\": \"$version\", \
  \"target_commitish\": \"master\", \
  \"name\": \"psqlpack-$version\", \
  \"body\": \"psqlpack $version\", \
  \"draft\": false, \
  \"prerelease\": true \
}"
id=$(curl -X POST -s \
    -H "Accept: application/vnd.github.v3+json" \
    -H "Authorization: token $GITHUB_TOKEN" \
    -H "Content-Type: application/json" \
    --data "$body" \
    "https://api.github.com/repos/paupino/psqlpack/releases" | jq -r '.id')

# And upload each asset
for target in ${targets[*]}
do
    filename="psqlpack-$target.tar.gz"
    asset="https://uploads.github.com/repos/paupino/psqlpack/releases/$id/assets?name=$(basename $filename)"
    echo $asset
    # POST /repos/:owner/:repo/releases/:release_id/assets?name=foo.zip
    curl -X POST -s \
        -H "Accept: application/vnd.github.v3+json" \
        -H "Authorization: token $GITHUB_TOKEN" \
        -H "Content-Type: application/gzip" \
        --data-binary @target/packages/psqlpack-$target.tar.gz \
        $asset | jq -r '.url'
    
done
