VERSION?=latest

build-arm:
	@echo "Building for aarch64"
	docker build -t trulyao/chimney:${VERSION} --build-arg ARCH=aarch64 --platform linux/arm64 .

build-x86:
	@echo "Building for x86_64"
	docker build -t trulyao/chimney:${VERSION} --build-arg ARCH=x86_64 --platform linux/amd64 .
