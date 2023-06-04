#!/bin/bash -x


systemctl --user stop rust-hyper-server.service

DONE=false

while [ $DONE = "false" ] ; do

  echo "begin loop $(date)"

  cd ~/rust-hyper-server

  git pull -v

  RELEASE=$(git describe --abbrev=0 --tags)
  echo "RELEASE=$RELEASE"

  rm -fr target
  mkdir -p target/release
  cd target/release

  URL="https://github.com/aaronriekenberg/rust-hyper-server/releases/download/${RELEASE}/rust-hyper-server-aarch64-unknown-linux-gnu.tar.gz"

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

tar xvf rust-hyper-server-aarch64-unknown-linux-gnu.tar.gz

sudo setcap cap_net_bind_service=+ep ./rust-hyper-server

systemctl --user restart rust-hyper-server.service
