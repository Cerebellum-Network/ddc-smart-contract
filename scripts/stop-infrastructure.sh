#!/usr/bin/env bash

# Kill all containers
docker kill $(docker ps -q)
docker rm -f $(docker ps -a -q)

