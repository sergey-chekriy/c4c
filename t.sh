cargo fmt
cargo clippy -- -D warnings
cargo test

rm -rf out
cargo run -- validate examples/internet-banking.dsl
cargo run -- inspect examples/internet-banking.dsl
cargo run -- export examples/internet-banking.dsl --format mermaid --out out
cat out/system-context.mmd
cat out/container.mmd


for f in tests/fixtures/m5-*.dsl; do
  echo "=== $f ==="
  cargo run -- validate "$f" || true
done

rm -rf out-m5
cargo run -- export tests/fixtures/m5-styles.dsl --format mermaid --out out-m5
find out-m5 -type f -maxdepth 1 -print -exec sed -n '1,160p' {} \;