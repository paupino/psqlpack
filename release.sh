#!/bin/bash
set -e

version=$1
if [[ ! $version =~ ^[0-9]+\.[0-9]+(\.[0-9])?$ ]]; then
	echo "Please enter a valid version: '$version'"
	exit 1
fi

# This is the array of targets to deploy
targets=(
    "x86_64-apple-darwin"
    #"x86_64-unknown-linux-musl"
)

# Build each target first
mkdir -p target/packages/
for target in ${targets[*]}
do
    # Create the binary first
    echo "Compiling $target"
    if [ "$target" == "x86_64-unknown-linux-musl" ]; then
      docker run -v $PWD:/volume --rm -t clux/muslrust cargo build --release
    else
      cargo build --target=$target --release
    fi
    cp -f target/$target/release/psqlpack target/packages/
    pushd target/packages/ > /dev/null
    tar -cvzf psqlpack-$target.tar.gz psqlpack > /dev/null
    popd > /dev/null
done

# First, delete the latest release 
echo "Deleting last release of $version"
to_delete=$(curl -s \
    -H "Accept: application/vnd.github.v3+json" \
    -H "Authorization: token $GITHUB_TOKEN" \
    "https://api.github.com/repos/paupino/psqlpack/releases/tags/$version" | jq -r '.url')
if [ ! $to_delete = 'null' ]; then 
    curl -X DELETE -s \
        -H "Accept: application/vnd.github.v3+json" \
        -H "Authorization: token $GITHUB_TOKEN" \
        $to_delete
fi

# Now we create the release in Github
# POST /repos/:owner/:repo/releases
echo "Creating release $version"
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
echo "   Uploading assets to $id..."
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
echo "Done!"