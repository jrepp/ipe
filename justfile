# Makefile owns command behavior. This file provides aliases.
default:
    @just --list

build:
    make build

release:
    make release

test:
    make test

test-default:
    make test-default

lint:
    make lint

fmt:
    make fmt

fmt-check:
    make fmt-check

check:
    make check

dev:
    make dev

docs:
    make docs

docs-site:
    make docs-site

docs-serve:
    make docs-serve

docs-check:
    make docs-check

docs-cms-check:
    make docs-cms-check

repo-check:
    make repo-check

clean:
    make clean

bench:
    make bench

perf:
    make perf

runtime-acceptance:
    make runtime-acceptance

policy-acceptance:
    make policy-acceptance

build-release:
    make release
