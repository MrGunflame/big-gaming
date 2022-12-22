CARGO := cargo

FEATURES := simd,tracing/release_max_level_info

.PHONY: build

all: build

build:
	$(CARGO) +nightly build --release --no-default-features --features=$(FEATURES)
