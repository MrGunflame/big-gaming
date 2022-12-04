
.PHONY: build

all: build

build:
	cargo +nightly build --release --no-default-features --features simd
