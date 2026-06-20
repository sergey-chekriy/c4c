make check
make grammar
make grammar-test
cargo run -- validate examples/internet-banking.dsl
cargo run -- export examples/internet-banking.dsl --format mermaid --out out