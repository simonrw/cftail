#!/bin/bash

set -eou pipefail

STACK_NAME=cftail-test-stack
TF_FILENAME=$(readlink -f $(dirname $0)/cloudformation.yml)

main() {
    aws cloudformation deploy --stack-name $STACK_NAME --template-file $TF_FILENAME
    aws cloudformation wait stack-create-complete --stack-name $STACK_NAME
}

main