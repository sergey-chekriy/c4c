# Roadmap

## M1 — prototype subset

A dependency-free Rust prototype that validates and exports a small but useful Structurizr DSL subset.

## M2 — industrial parser/diagnostics

Introduce formal lexer/parser, source spans, recoverable errors, and compiler-quality diagnostics.

## M3 — full core model grammar

Implement all workspace/model constructs from the language reference: workspace extension, docs/ADRs, properties, identifiers, implied relationships, enterprise, groups, generic elements, deployment environments/nodes/groups, infrastructure nodes, system/container instances, and relationship removal.

Status: implemented in the single-crate compiler; later-milestone behavior is preserved or rejected safely.

## M4 — full view grammar

Implement system landscape, system context, container, component, filtered, dynamic, deployment, custom, and image views.

Status: implemented with deterministic static/filtered/deployment expansion, Mermaid graph/sequence export, and explicit warnings for deferred rendering semantics.

## M4.5 — Tree-sitter syntax frontend

Use a committed Tree-sitter grammar as the syntax source of truth while preserving the Rust semantic model, validation, diagnostics, view expansion, and exporters.

Status: implemented behind the default parser facade, with the handwritten parser retained as the CST-to-semantic-model compatibility adapter until direct mapping reaches parity.

## M5 — style/theme layer

Implement styles, themes, branding, terminology, light/dark styling, element styles, and relationship styles.

Status: implemented with validated semantic metadata, safe offline references, terminology labels, and deterministic partial Mermaid styling.

## M6 — preprocessing and advanced DSL features

Implement includes, constants, substitution, expressions, scripts/plugins as safe parsed constructs, and explicit network opt-in for URL usage.

Status: implemented with source-mapped local includes, deterministic directories, constants, a safe selector-expression subset, and strict rejection without network or code execution.

## M6.5 — Tree-sitter packaging cleanup

Keep Node.js grammar-development-only while Rust builds compile committed generated artifacts.

Status: implemented with documented Make targets, committed generated artifacts, and GitHub Linguist metadata.

## M7 — documentation and ADRs

Import Markdown/AsciiDoc documentation and ADRs; generate static documentation site.

Status: implemented with secure local imports, adr-tools-compatible records, escaped static HTML, local Mermaid sources, and deterministic ADR listing.

## M8 — exporters

Support JSON, Mermaid, D2, PlantUML, C4-PlantUML, DOT, SVG/PNG, Draw.io, ArchiMate XML, and static HTML.

Status: implemented with deterministic text exporters, M7 site delegation, a conservative
ArchiMate 3.0 exchange mapping, and explicit local-only Graphviz rendering policy.

## M8.1 — Archi native exporter

Status: implemented with deterministic native `.archimate` XML, folders, elements,
relationships, diagram views, diagram objects, and connections. This Archi-specific format
is separate from the Open Group `archimate` exchange exporter.

## M8.3 — ArchiMate extension profile

M8.3 covers the ArchiMate vocabulary listed in the ArchiMate 3.1 reference cards, plus basic formatting and Archi export/import mapping.

Status: implemented with explicit ArchiMate element/relationship types, formatting metadata, `archimateView` support, and Archi-native/Open Group export mappings. Full formal ArchiMate conformance, including the relationship validity matrix, derivation rules, viewpoints, and advanced semantic validation, is deferred.

## M9 — compatibility suite

Add fixture coverage for every language construct and compare outputs against expected snapshots.

## M10 — LSP/editor support

Local-only LSP for diagnostics, completion, hover, references, rename, semantic tokens, and preview support.

## M11 — Archi/ArchiMate bridge

Optional ArchiMate import and richer view/layout bridge while keeping ArchiMate semantics outside the C4 compiler core.

Status: initial native bridge implemented with safe XML parsing, C4 projection, lossless sidecar
round-tripping for unchanged projections, canonical diffing, and connection integrity validation.
Semantic merging of edited C4 projections into existing native diagrams remains deferred.

## M12 — release engineering

CI, cross-platform binaries, Homebrew, checksums, SBOM, and supply-chain review.
