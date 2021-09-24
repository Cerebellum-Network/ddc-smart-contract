#!/usr/bin/env bash

# Setup versions on images
export POS_NODE_VERSION="${1:-latest}"
export DDC_E2E_TEST_VERSION="${2:-latest}"

# Copy artifacts of DDC SC
. ./scripts/copy-ddc-sc.sh

# Start infrastructure
. ./scripts/start-infrastructure.sh

# Pull ddc-e2e-tests image
docker pull "338287888375.dkr.ecr.us-west-2.amazonaws.com/ddc-e2e-tests:$DDC_E2E_TEST_VERSION"

# Run tests (deploy ddc smart contract on pos-network-node)
# ToDo delete dependency , https://cerenetwork.atlassian.net/browse/CBI-1418
docker run --rm \
  -e NODE_URL_1="ws://${BOOT_NODE_1}:9944" \
  -e NODE_URL_2="ws://${BOOT_NODE_2}:9945" \
  -v "$PWD/artifacts":/blockchain-tests/artifacts \
  --network=test-net \
  --entrypoint '/bin/sh' "338287888375.dkr.ecr.us-west-2.amazonaws.com/ddc-e2e-tests:$DDC_E2E_TEST_VERSION" -c 'npm run deploy-ddc-sc'

# Stop and remove all containers
. ./scripts/stop-infrastructure.sh
