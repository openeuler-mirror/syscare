#!/bin/bash -e

readonly REPO_NAME="syscare"
readonly REPO_URL="https://gitee.com/openeuler/$REPO_NAME"

# Prepare
repo_version=$(grep "Version" "$REPO_NAME.spec" | head -n 1 | awk -F ' ' '{print $NF}')
repo_dir="$REPO_NAME-$repo_version"

rm -rf "$REPO_NAME" "$repo_dir"
git clone "$REPO_URL"

# Prepare package build requirements 
pushd "$REPO_NAME"

cargo update -p clap --precise 4.0.32
cargo update -p clap_lex --precise 0.3.0
cargo vendor

mkdir -p .cargo
cat << EOF > .cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

popd

# Create tarball
mv "$REPO_NAME" "$repo_dir"
tar -czvf "$repo_dir.tar.gz" --exclude-vcs "$repo_dir"

# Clean up
rm -rf "$repo_dir"

