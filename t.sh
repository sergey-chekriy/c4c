cargo fmt
cargo clippy -- -D warnings
cargo test

for fmt in json mermaid d2 plantuml c4plantuml dot drawio archimate html; do
  rm -rf "out-$fmt"
  echo "=== $fmt ==="
  cargo run -- export tests/fixtures/m8-exporters.dsl --format "$fmt" --out "out-$fmt"
  find "out-$fmt" -maxdepth 2 -type f | sort
done

cargo run -- export tests/fixtures/m8-exporters.dsl --format svg --out out-svg || true
cargo run -- export tests/fixtures/m8-exporters.dsl --format png --out out-png || true