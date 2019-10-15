#!/usr/bin/env bash

if [ ! -f ./marta.env ]; then
   echo "Please set MARTA_TRAIN_API_KEY= in marta.env"
   exit 1
fi

if [ $UID -ne 0 ]; then
    echo "Install script requires root permissions"
    exit 1
fi

set -x

install -C ./marta.env /usr/local/etc/marta.env
install -C ./target/release/warp_proxy /usr/local/bin/warp_proxy
install -C ./warp-proxy.service /etc/systemd/system/warp-proxy.service

systemctl daemon-reload

