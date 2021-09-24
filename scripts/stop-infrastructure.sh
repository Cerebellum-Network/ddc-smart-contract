#!/usr/bin/env bash

# Kill all containers
docker kill $(docker ps -q)
docker rm -f $(docker ps -a -q)

# Delete chain-data folder if exist
chmod -R 777 "pos-node/chain-data1"
chmod -R 777 "pos-node/chain-data2"
[ -d "pos-node/chain-data1" ] && rm -r "pos-node/chain-data1"
[ -d "pos-node/chain-data2" ] && rm -r "pos-node/chain-data2"

