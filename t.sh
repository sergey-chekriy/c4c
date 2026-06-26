cargo fmt
cargo clippy -- -D warnings
cargo test
make grammar-test

cargo run -- validate tests/fixtures/m83-archimate-profile.dsl
cargo run -- inspect tests/fixtures/m83-archimate-profile.dsl