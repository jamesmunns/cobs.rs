all: check build embedded test clippy fmt docs coverage

clippy:
  cargo clippy --all-targets -- -D warnings

fmt:
  cargo fmt --all -- --check

check:
  cargo check --all-features

embedded:
  cargo build --target thumbv7em-none-eabihf --no-default-features

test:
  cargo nextest r --all-features
  cargo nextest r --no-default-features
  cargo nextest r --no-default-features --features alloc
  cargo test --doc

build:
  cargo build --all-features

docs:
  RUSTDOCFLAGS="--cfg docsrs -Z unstable-options --generate-link-to-definition" cargo +nightly doc --all-features

docs-html:
  RUSTDOCFLAGS="--cfg docsrs -Z unstable-options --generate-link-to-definition" cargo +nightly doc --all-features --open

coverage:
  cargo llvm-cov nextest

coverage-html:
  cargo llvm-cov nextest --html --open
