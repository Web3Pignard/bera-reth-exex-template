#!/usr/bin/make -f

###############################################################################
###                               Variables                                 ###
###############################################################################

GIT_SHA ?= $(shell git rev-parse HEAD)
GIT_TAG ?= $(shell git describe --tags --abbrev=0 2>/dev/null || echo "dev")
DOCKER_IMAGE_NAME ?= bera-reth-exex-template

###############################################################################
###                               Docker                                    ###
###############################################################################

.PHONY: docker-build
docker-build: ## Build production Docker image with maxperf profile
	docker build --tag $(DOCKER_IMAGE_NAME):latest \
		--build-arg COMMIT=$(GIT_SHA) \
		--build-arg VERSION=$(GIT_TAG) \
		--build-arg BUILD_PROFILE=maxperf \
		--build-arg FEATURES="jemalloc asm-keccak min-debug-logs" \
		.

###############################################################################
###                           Local Development                             ###
###############################################################################

.PHONY: start-local
start-local: ## Start ExEx with BeaconKit integration
	@echo "Starting local development environment..."
	./scripts/start-local.sh

###############################################################################
###                               Help                                      ###
###############################################################################

.PHONY: help
help: ## Display this help message
	@echo "Bera-Reth ExEx Template"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.DEFAULT_GOAL := help