#!/bin/bash -x

if [ $# -ne 1 ]; then
  echo "usage: $0 <release version>"
  exit 1
fi

RELEASE_VERSION=$1
RELEASE_VERSION_WITHOUT_V=$(echo $RELEASE_VERSION | sed -e 's/^v//g')

echo "RELEASE_VERSION=$RELEASE_VERSION"
echo "RELEASE_VERSION_WITHOUT_V=$RELEASE_VERSION_WITHOUT_V"

cd ~/rust-hyper-unixh2c

toml set Cargo.toml package.version $RELEASE_VERSION_WITHOUT_V > Cargo.toml.tmp
mv Cargo.toml.tmp Cargo.toml

cargo build -v
RESULT=$?
echo "cargo build RESULT = $RESULT"
if [ $RESULT -ne 0 ]; then
  echo "cargo build failed"
fi

cargo test -v
RESULT=$?
echo "cargo test RESULT = $RESULT"
if [ $RESULT -ne 0 ]; then
  echo "cargo test failed"
fi

git add Cargo.toml Cargo.lock

git commit -m "Version $RELEASE_VERSION_WITHOUT_V"

git tag $RELEASE_VERSION

git push -v
git push -v --tags
