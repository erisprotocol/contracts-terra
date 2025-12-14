#!/usr/bin/env bash

set -e
set -o pipefail

# projectPath=/c/Projects/eris/liquid-staking-contracts
projectPath=$(dirname `pwd`) 
folderName=$(basename $(dirname `pwd`)) 

mkdir -p "../../$folderName-cache"
mkdir -p "../../$folderName-cache/target"
mkdir -p "../../$folderName-cache/registry"


if [ "$1" == "TESTNET" ]
then
  echo "Applying TESTNET" 
  sed -i 's/WEEK: u64 = 7 \* 86400/WEEK: u64 = 60 \* 60/' "$projectPath/packages/eris/src/governance_helper.rs"
else
  echo "Applying $1"
  sed -i 's/WEEK: u64 = 60 \* 60/WEEK: u64 = 7 \* 86400/' "$projectPath/packages/eris/src/governance_helper.rs"
fi

docker run --env $1 --rm -v "/$projectPath":/code \
  --mount type=bind,source=/$projectPath-cache/target,target=/target \
  --mount type=bind,source=/$projectPath-cache/registry,target=/usr/local/cargo/registry \
  --mount type=bind,source=/$projectPath-cache,target=/usr/local/cargo/git \
  --dns 8.8.8.8 --dns 8.8.4.4 \
  cosmwasm/optimizer:0.16.0