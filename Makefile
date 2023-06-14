PROJECT=simkube
DOCKER_REGISTRY=localhost:5000

MANIFESTS:=$(wildcard manifests/*.yml)

.PHONY: build test image deploy run

build:
	CGO_ENABLED=0 go build -trimpath -o output/${PROJECT} main.go

test:
	go test

image:
	docker build output -f images/Dockerfile -t ${DOCKER_REGISTRY}/${PROJECT}:latest
	docker push ${DOCKER_REGISTRY}/${PROJECT}:latest

.applied: ${MANIFESTS}
	@echo $? | xargs -d' ' -L1 kubectl apply -f 
	@touch $@

run: .applied
	kubectl rollout restart deployment ${PROJECT}
