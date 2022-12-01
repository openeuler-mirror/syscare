#!/bin/bash
set -e

REPO=$1
VERSION=$2

if [ -z "${REPO}" ]; then
	echo "Please assign a directory of syscare repo."
	exit 1
fi

if [ -z "${VERSION}" ]; then
	VERSION=HEAD
fi

cd ${REPO}
REPO=$(pwd)
REPO=${REPO##*/}
cd -
cp -r ${REPO}  ${REPO}-${VERSION}
cd ${REPO}-${VERSION}

if [ "${VERSION}" != "HEAD" ]; then
	git checkout v${VERSION}
else
	git checkout ${VERSION}
fi

for file in `find . -name Cargo.toml`
do
	dir=${file%/Cargo.toml*}

	if [ -n "${dir}" ]; then
		cd ${dir}
		cargo vendor
		mkdir -p .cargo

		cat > .cargo/config << EOF
[source.crates-io]
replace-with = "local-registry"

[source.local-registry]
directory = "vendor"
EOF

		cd -
	fi
done

cd ..

rm -rf ${REPO}-${VERSION}/.git
tar -zcf ${REPO}-${VERSION}.tar.gz ${REPO}-${VERSION}
echo "Output: ${REPO}-${VERSION}.tar.gz"

