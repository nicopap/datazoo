CLIPPY_ARGS=-- --deny clippy::all --deny clippy::pedantic --deny clippy::nursery \
	--allow clippy::use-self
.PHONY: check run pre-hook

check:
	cargo clippy

pre-hook:
	cargo test --no-default-features
	cargo clippy --no-default-features $(CLIPPY_ARGS)
	cargo test --all-features
	cargo clippy --all-features $(CLIPPY_ARGS)
	cargo test
	cargo clippy $(CLIPPY_ARGS)
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
	cargo fmt --all -- --check
