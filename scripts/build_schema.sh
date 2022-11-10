#!/usr/bin/env bash

set -e
set -o pipefail

projectPath=$(cd "$(dirname "${0}")" && cd ../ && pwd)

for c in "$projectPath"/contracts/*; do
  if [[ "$c" != *"amp-compounder" ]]; then
    if [[ "$c" != *"amp-governance" ]]; then
      (cd $c && cargo schema)
    fi
  fi
done

for c in "$projectPath"/contracts/amp-compounder/*; do
  if [[ "$c" != *"README.md" ]]; then
    (cd $c && cargo schema)
  fi
done

for c in "$projectPath"/contracts/amp-governance/*; do
  if [[ "$c" != *"README.md" ]]; then
    (cd $c && cargo schema)
  fi
done

# for c in "$projectPath"/contracts/periphery/*; do
#   (cd $c && cargo schema)
# done
