#!/usr/bin/env bash

export dir_path_to="${1:-./}"
export image_tag="${2:-latest}"
docker pull 338287888375.dkr.ecr.us-west-2.amazonaws.com/ddc-smart-contract:"$image_tag"
id=$(docker create "338287888375.dkr.ecr.us-west-2.amazonaws.com/ddc-smart-contract:$image_tag")
docker cp "${id}":/ddc-smart-contract/artifacts/ "$dir_path_to"
docker rm -v "$id"
