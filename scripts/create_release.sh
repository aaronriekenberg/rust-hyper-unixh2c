#!/bin/bash -x -e

if [ $# -ne 1 ]; then
  echo "usage: $0 <release version>"
  exit 1
fi

RELEASE_VERSION=$1

if [[ $RELEASE_VERSION != v* ]]; then
  echo "RELEASE_VERSION $RELEASE_VERSION does not begin with 'v'"
  exit 1
fi

RELEASE_VERSION_WITHOUT_V=$(echo $RELEASE_VERSION | sed -e 's/^v//g')

echo "RELEASE_VERSION=$RELEASE_VERSION"
echo "RELEASE_VERSION_WITHOUT_V=$RELEASE_VERSION_WITHOUT_V"

cd ~/vscode/rust-hyper-server

toml set Cargo.toml package.version $RELEASE_VERSION_WITHOUT_V > Cargo.toml.tmp

mv Cargo.toml.tmp Cargo.toml

cargo build -v

cargo test -v

git add Cargo.toml Cargo.lock

git commit -m "Version $RELEASE_VERSION_WITHOUT_V"

git tag $RELEASE_VERSION

git push -v
git push -v --tags
