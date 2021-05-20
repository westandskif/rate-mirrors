.PHONY: build_with_glibc build_with_musl

build_with_glibc:
	cargo build --release --locked --target=x86_64-unknown-linux-gnu

build_with_musl:
	cargo build --release --locked --target=x86_64-unknown-linux-musl
