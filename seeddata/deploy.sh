#!/bin/bash

set -eou pipefail

STACK_NAME=cftail-test-stack
if command -v realpath >/dev/null 2>/dev/null; then
    TF_FILENAME=$(realpath $(dirname $0)/cloudformation.yml)
else
    TF_FILENAME=$(readlink -f $(dirname $0)/cloudformation.yml)
fi

main() {
    aws cloudformation deploy --stack-name $STACK_NAME --template-file $TF_FILENAME
    aws cloudformation wait stack-create-complete --stack-name $STACK_NAME
}

main
