#!/usr/bin/env bash

echo "-------------   POS_NODE_VERSION:   $POS_NODE_VERSION"
docker-compose -f pos-node/docker-compose.boot.node.yaml pull
docker-compose -f pos-node/docker-compose.boot.node.yaml up -d
sleep 5;
export BOOT_NODE_1=$(docker inspect --format='{{(index .NetworkSettings.Networks "test-net").IPAddress}}' boot_node)
echo "-------------   BOOT_NODE_1:   $BOOT_NODE_1"
export NETWORK_IDENTIFIER=$(curl localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{  "jsonrpc":"2.0", "id":1, "method":"system_localPeerId", "params": []}' | jq -r ".result")
echo "-------------   NETWORK_IDENTIFIER:   $NETWORK_IDENTIFIER"
docker-compose -f pos-node/docker-compose.custom.node.yaml pull
docker-compose -f pos-node/docker-compose.custom.node.yaml up -d
sleep 5;
export BOOT_NODE_2=$(docker inspect --format='{{(index .NetworkSettings.Networks "test-net").IPAddress}}' validation_node_custom)
echo "-------------   BOOT_NODE_2:   $BOOT_NODE_2"

