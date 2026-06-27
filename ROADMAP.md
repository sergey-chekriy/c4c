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

Status: implemented with explicit ArchiMate element/relationship types, formatting metadata, `archimateView` support, and Archi-native/Open Group export mappings.

## M8.4 — ArchiMate semantic conformance

M8.4 adds practical semantic conformance for the explicit ArchiMate extension profile: central type registry metadata, element layer/category classification, relationship validation, junctions, AccessRelationship direction, viewpoint metadata, strict-mode errors, and improved native/Open Group export semantics.

Status: implemented as a practical validation/export layer, not official ArchiMate certification or a complete formal relationship matrix.

## M8.5 — ArchiMate viewpoints, notation, and layout

M8.5 improves generated native Archi diagrams with viewpoint-aware layout, fallback names, non-overlap placement, safer notation/style mapping, and clearer deterministic connection routing.

Status: implemented for generated native Archi output. Full native Archi round-trip fidelity is covered by M8.6.

## M8.6 — Archi native round-trip fidelity

M8.6 imports native Archi `.archimate` files into explicit c4c ArchiMate DSL plus a JSON sidecar, exports unchanged projections back to native Archi XML, and provides canonical native diffing for round-trip checks.

Status: implemented with safe native Archi XML parsing, extended DSL projection, lossless sidecar preservation for unchanged projections, sidecar-aware native export, canonical native diff, semantic diff, and local private-model workflow support. Full edited-projection sidecar merging remains deferred.

## M8.7 — ArchiMate full conformance hardening

M8.7 hardens the ArchiMate profile with broader viewpoint metadata, conservative relationship matrix warnings, stricter `--strict` behavior, native Archi compatibility checks, sidecar/no-sidecar regression coverage, and Open Group export checks.

Status: implemented as practical conformance hardening. The checker is local and text-only; it does not execute Archi, fetch schemas, or claim official certification-level validation.

## M8.8 — ArchiMate 3.2 alignment

M8.8 aligns the implemented ArchiMate baseline with locally available ArchiMate 3.2 reference cards, adds version reporting, conformance documentation, and 3.2 vocabulary/export regression tests.

Status: implemented as practical ArchiMate 3.2 support. No implementation-impacting vocabulary or relationship deltas were identified from the available 3.2 reference cards; formal certification remains out of scope.

## M8.9 — ArchiMate 4 readiness and version strategy

Define versioned vocabulary/export strategy before implementing ArchiMate 4 changes.

## M8.10 — ArchiMate 4 implementation alignment

Implement verified ArchiMate 4 deltas when local references and compatibility strategy are available.

## M8.11 — Open Group Exchange Format compliance

Harden standards-oriented exchange output/import against official exchange-format requirements without using network schema fetching.

## M8.12 — Conformance matrix + interoperability suite

Add comprehensive matrix fixtures and interoperability snapshots for supported ArchiMate/C4 workflows.

## M8.13 — Certification readiness package

Prepare evidence, gaps, and process docs for future certification work; no certification claim is made before this.

## M9 — compatibility suite

Add fixture coverage for every language construct and compare outputs against expected snapshots.

## M10 — LSP/editor support

Local-only LSP for diagnostics, completion, hover, references, rename, semantic tokens, and preview support.

## M11 — Archi/ArchiMate bridge

Optional ArchiMate import and richer view/layout bridge while keeping ArchiMate semantics outside the C4 compiler core.

Status: initial native bridge implemented with safe XML parsing, explicit ArchiMate DSL projection, lossless sidecar
round-tripping for unchanged projections, canonical diffing, and connection integrity validation.
Semantic merging of edited C4 projections into existing native diagrams remains deferred.

## M12 — release engineering

CI, cross-platform binaries, Homebrew, checksums, SBOM, and supply-chain review.
