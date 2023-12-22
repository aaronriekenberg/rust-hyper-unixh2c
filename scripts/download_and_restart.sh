#!/bin/bash -x

DONE=false

PROJECT_PATH=$(realpath $(dirname $0)/..)
echo "PROJECT_PATH = $PROJECT_PATH"

cd $PROJECT_PATH

while [ $DONE = "false" ] ; do

  echo "begin loop $(date)"

  git pull -v

  RELEASE=$(git describe --abbrev=0 --tags)
  echo "RELEASE=$RELEASE"

  URL="https://github.com/aaronriekenberg/rust-hyper-server/releases/download/${RELEASE}/rhs-$(arch)-unknown-linux-gnu.tar.gz"

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

systemctl --user stop rust-hyper-server.service

rm -fr target
mkdir -p target/release
cd target/release

mv $PROJECT_PATH/rhs-$(arch)-unknown-linux-gnu.tar.gz .
tar xvf rhs-$(arch)-unknown-linux-gnu.tar.gz

#sudo setcap cap_net_bind_service=+ep ./rhs

systemctl --user restart rust-hyper-server.service
