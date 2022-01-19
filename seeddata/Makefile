STACK_NAME := cftail-test-stack

deploy:
	bash ./deploy.sh

delete:
	aws cloudformation delete-stack --stack-name ${STACK_NAME}
	aws cloudformation wait stack-delete-complete --stack-name ${STACK_NAME}
