CARGO := cargo
TARGET := dist

.PHONY: build

all: build

build:
	cd .. && $(CARGO) build --release --bin=make
	cd .. && ./target/release/make --target-dir=$(TARGET)
