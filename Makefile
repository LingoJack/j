SHELL := /bin/bash

.PHONY: current_dir
current_dir:
	@echo "Current directory: $$(pwd)"

.PHONY: push
push: current_dir
	@git add .\
	&& (git commit -m "新增了一些特性" || exit 0) \
	&& git push origin main

.PHONY: pull
pull: current_dir
	@git pull origin main

status: current_dir
	@git status

release: current_dir
	@cargo build --release