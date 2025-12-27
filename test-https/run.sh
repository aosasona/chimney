#!/bin/bash
# Quick script to run Chimney HTTPS test server
#
# Usage: ./run.sh [--sudo]

set -e

cd "$(dirname "$0")/.."

echo "🔧 Building Chimney..."
cargo build --release

echo ""
echo "📋 Test Configuration:"
echo "  - HTTP:  http://localhost:8080 → redirects to HTTPS"
echo "  - HTTPS: https://localhost (self-signed cert)"
echo "  - Config: test-https/chimney.toml"
echo ""
echo "⚠️  You'll need to accept the browser security warning for the self-signed certificate"
echo ""

if [[ "$1" == "--sudo" ]] || [[ "$EUID" -ne 0 ]]; then
    echo "🚀 Starting server with sudo (port 443 requires privileges)..."
    echo "   If you don't want to use sudo, grant capabilities:"
    echo "   sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/chimney"
    echo ""
    sudo ./target/release/chimney --config test-https/chimney.toml
else
    echo "🚀 Starting server..."
    ./target/release/chimney --config test-https/chimney.toml
fi
