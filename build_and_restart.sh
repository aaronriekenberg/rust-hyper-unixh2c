#!/bin/bash -x

cd ~/rust-hyper-notcgi

systemctl --user stop rust-hyper-notcgi.service

git pull -v

time cargo build -v --release
RESULT=$?
if [ $RESULT -ne 0 ]; then
  echo "cargo build failed RESULT = $RESULT"
  exit $RESULT
fi

systemctl --user restart rust-hyper-notcgi.service
