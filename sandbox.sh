#!/usr/bin/env bash
set -e

./build.sh
hc sandbox generate ./happ --run=8888