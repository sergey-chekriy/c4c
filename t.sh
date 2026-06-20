rm -rf out
cargo run -- validate examples/internet-banking.dsl
cargo run -- inspect examples/internet-banking.dsl
cargo run -- export examples/internet-banking.dsl --format mermaid --out out
cat out/system-context.mmd
cat out/container.mmd

