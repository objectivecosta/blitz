#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

if docker image inspect blitz_cross_compiler >/dev/null 2>&1; then
    echo "Image already exists locally"
else
    echo "Image does not exist locally. Creating..."
    docker build -t blitz_cross_compiler .;
fi

docker run -v ./:/container/project blitz_cross_compiler