#!/bin/bash -e

readonly REPO_NAME="syscare"
readonly REPO_PROVIDER="openeuler"
readonly REPO_URL="https://gitee.com/$REPO_PROVIDER/$REPO_NAME"
readonly REPO_BRANCH="openEuler-20.03"

echo "Cloning source code..."
repo_version=$(grep "Version" "$REPO_NAME.spec" | head -n 1 | awk -F ' ' '{print $NF}')
repo_dir="$REPO_NAME-$repo_version"

rm -rf "$REPO_NAME" "$repo_dir"
git clone "$REPO_URL"

echo "Prepare build requirements..."
pushd "$REPO_NAME"

echo "Checking out dest branch..."
git checkout "$REPO_BRANCH"

echo "Vendoring dependencies..."
cargo vendor --respect-source-config --sync upatch/Cargo.toml

mkdir -p .cargo
cat << EOF > .cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

popd

echo "Compressing package..."
mv "$REPO_NAME" "$repo_dir"
tar -czf "$repo_dir.tar.gz" "$repo_dir"

echo "Cleaning up..."
rm -rf "$repo_dir"

echo "Done"

