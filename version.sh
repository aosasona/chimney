#!/bin/bash

PATH="$HOME/.cargo/bin:$PATH"
VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].version')

# IF show is passed as an argument, print the version
if [[ "$1" == "show" ]]; then
	echo "$VERSION"
fi

export VERSION
