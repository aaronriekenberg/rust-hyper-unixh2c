#!/bin/bash -x


systemctl --user stop rust-hyper-unixh2c.service

DONE=false

while [ $DONE = "false" ] ; do

  cd ~/rust-hyper-unixh2c

  git pull -v

  RELEASE=$(git describe --abbrev=0 --tags)
  echo "RELEASE=$RELEASE"

  rm -fr target
  mkdir -p target/release
  cd target/release

  URL="https://github.com/aaronriekenberg/rust-hyper-unixh2c/releases/download/${RELEASE}/rust-hyper-unixh2c-aarch64-unknown-linux-gnu.tar.gz"

  wget $URL
  WGET_RESULT=$?
  if [ $WGET_RESULT -eq 0 ] ; then
    DONE=true
  else
    echo "wget failure result $WGET_RESULT sleeping"
    DONE=false
    sleep 120
  fi

done

tar xvf rust-hyper-unixh2c-aarch64-unknown-linux-gnu.tar.gz

systemctl --user restart rust-hyper-unixh2c.service
