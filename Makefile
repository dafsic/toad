SHELL=/usr/bin/env bash

PROJECT:=github.com/dafsic/toad

BINDIR     := $(CURDIR)/bin
GO_VERSION := $(shell go version)
BUILD_TIME := $(shell date +%Y%m%d%H%M%S)
#BUILD_TIME := $(shell date +%Y-%m-%dT%H:%M:%S%z)

# go option
PKG         := ./...
TESTS       := .
TESTFLAGS   :=
CGO_ENABLED ?= 0
GO_LDFLAGS += -s -w

COMMIT_HASH := $(shell git rev-parse --short=8 HEAD || echo unknown)
GIT_BRANCH  := $(shell git rev-parse --abbrev-ref HEAD)
GIT_DIRTY   := $(shell test -n "`git status --porcelain`" && echo "dirty" || echo "clean")
GIT_TAG     := $(shell git describe --tags --abbrev=0 --exact-match 2>/dev/null)
#GIT_TAG     := $(shell git describe --tags `git rev-list --tags --max-count=1`)

GO_LDFLAGS += -X '$(PROJECT)/app.build_time=$(BUILD_TIME)'
GO_LDFLAGS += -X '$(PROJECT)/app.go_version=$(GO_VERSION)'
GO_LDFLAGS += -X '$(PROJECT)/app.commit_hash=$(COMMIT_HASH)'
GO_LDFLAGS += -X '$(PROJECT)/app.git_branch=$(GIT_BRANCH)'
GO_LDFLAGS += -X '$(PROJECT)/app.version=$(GIT_TAG)'
GO_LDFLAGS += -X '$(PROJECT)/app.git_tree_state=$(GIT_DIRTY)'

.PHONY: default
default: check proto gateway telegram kraken_grid

# --------------------------------------------------------------------------------
# compile

.PHONY: check
check: ## Check working tree is clean or not
ifneq ($(shell git status -s),)
	$(error You must run git commit)
endif

.PHONY: gateway
gateway:  ## Compile gateway
	CGO_ENABLED=$(CGO_ENABLED) go build -trimpath -ldflags "$(GO_LDFLAGS)" -o $(BINDIR)/gateway ./gateway/cmd

.PHONY: kraken_grid
kraken_grid:  ## Compile kraken_grid
	CGO_ENABLED=$(CGO_ENABLED) go build -trimpath -ldflags "$(GO_LDFLAGS)" -o $(BINDIR)/kraken_grid ./kraken_grid/cmd

.PHONY: telegram
telegram:  ## Compile telegram
	CGO_ENABLED=$(CGO_ENABLED) go build -trimpath -ldflags "$(GO_LDFLAGS)" -o $(BINDIR)/telegram ./telegram/cmd

.PHONY: proto
proto: ## Generate proto files
	protoc --go_out=proto_go/kraken_grid --go-grpc_out=proto_go/kraken_grid  proto/kraken_grid/server.proto

# --------------------------------------------------------------------------------
# help

.PHONY: help
help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'