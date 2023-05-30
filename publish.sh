#!/bin/bash
set -ex
here=$(realpath $(dirname "$0"))
cd "$here"


if [ -z ${1+x} ] ; then
    echo "missing tag"
    exit 1
fi

TAG=$1
BUILD_MODE=${2:-release}

docker build -t protryon/nyauser:$TAG --build-arg BUILD_MODE=$BUILD_MODE -f ./Dockerfile .
docker push protryon/nyauser:$TAG
docker image rm protryon/nyauser:$TAG

cd $here/nyauser-types
cargo publish

cd $here/nyauser
cargo publish

cd $here/nyauser-cli
cargo publish