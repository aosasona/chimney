#!/bin/bash

export VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].version')
