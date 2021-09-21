#!/usr/bin/env bash

# Delete and create chain-data folder
[ -d "pos-node/chain-data1" ] && rm -r "pos-node/chain-data1"
[ -d "pos-node/chain-data2" ] && rm -r "pos-node/chain-data2"
mkdir "pos-node/chain-data1"
mkdir "pos-node/chain-data2"
chmod -R 777 "pos-node/"

# Start POS nodes
echo "-------------   POS_NODE_VERSION:   $POS_NODE_VERSION"
docker-compose -f pos-node/docker-compose.boot-node.yaml pull
docker-compose -f pos-node/docker-compose.boot-node.yaml up -d
sleep 5;
export BOOT_NODE_1=$(docker inspect --format='{{(index .NetworkSettings.Networks "test-net").IPAddress}}' boot_node)
echo "-------------   BOOT_NODE_1:   $BOOT_NODE_1"
export NETWORK_IDENTIFIER=$(curl localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{  "jsonrpc":"2.0", "id":1, "method":"system_localPeerId", "params": []}' | jq -r ".result")
echo "-------------   NETWORK_IDENTIFIER:   $NETWORK_IDENTIFIER"
docker-compose -f pos-node/docker-compose.validator-node.yaml pull
docker-compose -f pos-node/docker-compose.validator-node.yaml up -d
sleep 5;
export BOOT_NODE_2=$(docker inspect --format='{{(index .NetworkSettings.Networks "test-net").IPAddress}}' validator_node)
echo "-------------   BOOT_NODE_2:   $BOOT_NODE_2"

