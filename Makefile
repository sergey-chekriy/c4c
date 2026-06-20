.PHONY: grammar grammar-test check

grammar:
	cd tree-sitter-structurizr-dsl && npm run generate

grammar-test:
	cd tree-sitter-structurizr-dsl && npm test

check:
	cargo fmt
	cargo clippy -- -D warnings
	cargo test
