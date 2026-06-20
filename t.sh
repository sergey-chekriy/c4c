cargo fmt
cargo clippy -- -D warnings
cargo test

rm -rf out-site
cargo run -- docs tests/fixtures/m7-docs.dsl --out out-site
cargo run -- adr list tests/fixtures/m7-docs.dsl
find out-site -maxdepth 3 -type f | sort