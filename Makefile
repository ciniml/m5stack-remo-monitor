.PHONY: all build-device build-linux run-device run-linux clean clean-device clean-linux

all: build-device

build-device:
	cargo build --release

run-device:
	cargo run --release

build-linux:
	cargo +stable build --target x86_64-unknown-linux-gnu

run-linux:
	cargo +stable run --target x86_64-unknown-linux-gnu

clean: clean-device clean-linux

clean-device:
	cargo clean

clean-linux:
	cargo +stable clean --target x86_64-unknown-linux-gnu
