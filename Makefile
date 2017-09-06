SHELL := /bin/bash

build:
	@pushd cli && cargo build && popd