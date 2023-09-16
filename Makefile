CLIPPY_ARGS=-- --deny clippy::all --deny clippy::pedantic --deny clippy::nursery --deny missing_docs
.PHONY: check run pre-hook

check:
	cargo clippy --no-default-features $(CLIPPY_ARGS)
	cargo clippy --all-features $(CLIPPY_ARGS)

pre-hook:
	cargo test --no-default-features
	cargo clippy --no-default-features $(CLIPPY_ARGS)
	cargo test --all-features
	cargo clippy --all-features $(CLIPPY_ARGS)
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
	cargo fmt -- --check
