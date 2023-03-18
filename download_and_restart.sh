#!/bin/bash -x

cd ~/rust-hyper-unixh2c

systemctl --user stop rust-hyper-unixh2c.service

git pull -v

RELEASE=$(git describe --abbrev=0 --tags)
echo "RELEASE=$RELEASE"

rm -fr target
mkdir -p target/release
cd target/release

wget https://github.com/aaronriekenberg/rust-hyper-unixh2c/releases/download/${RELEASE}/rust-hyper-unixh2c-aarch64-unknown-linux-gnu.tar.gz

tar xvf rust-hyper-unixh2c-aarch64-unknown-linux-gnu.tar.gz

systemctl --user restart rust-hyper-unixh2c.service
