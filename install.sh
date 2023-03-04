#!/bin/bash
set -e

. ~/.cargo/env
cargo build --release
systemctl stop nyauser
cp ./target/release/nyauser /usr/local/bin/nyauser
cp ./assets/nyauser.service /etc/systemd/system/nyauser.service
cp ./config.yml /etc/nyauser.yml

systemctl daemon-reload
systemctl restart nyauser
