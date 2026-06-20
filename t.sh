cargo fmt
cargo clippy -- -D warnings
cargo test

cargo run -- validate tests/fixtures/m6-preprocessing.dsl
cargo run -- validate tests/fixtures/m6-nested.dsl

for f in \
  tests/fixtures/m6-cycle-a.dsl \
  tests/fixtures/m6-missing.dsl \
  tests/fixtures/m6-remote.dsl \
  tests/fixtures/m6-unsafe.dsl
do
  echo "=== SHOULD FAIL SAFELY: $f ==="
  cargo run -- validate "$f" && exit 1 || echo "OK: failed safely"
done