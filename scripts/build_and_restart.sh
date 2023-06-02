#!/bin/bash -x

cd ~/rust-hyper-server

systemctl --user stop rust-hyper-server.service

git pull -v

time cargo build -v --release -j2
RESULT=$?
if [ $RESULT -ne 0 ]; then
  echo "cargo build failed RESULT = $RESULT"
  exit $RESULT
fi

systemctl --user restart rust-hyper-server.service
