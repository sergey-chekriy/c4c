# c4c — local Structurizr DSL compiler prototype

`c4c` is a Rust prototype for a local/offline Structurizr DSL compiler.

Goal: parse Structurizr-style C4 architecture-as-code, validate it locally, and export static artifacts without SaaS, remote rendering, accounts, or telemetry.

## Current scope

Implemented in this repo:

- CLI with `validate`, `inspect`, `export`, `docs`, and `adr list` commands.
- Tree-sitter grammar as the syntax source of truth, with the proven handwritten parser retained as the semantic-model adapter during parity migration.
- Workspace parsing with optional name and description.
- `!identifiers hierarchical`.
- `model` block.
- `person`.
- `softwareSystem` with nested `container`.
- `container` with optional description and technology.
- `component` data model and view support.
- Relationships using `->` with optional description and technology.
- `views` block.
- System landscape, system context, container, component, filtered, dynamic, deployment, custom, and image view grammar.
- Deterministic static/filtered/deployment expansion and Mermaid graph export; dynamic views export as Mermaid sequences.
- Multi-value include/exclude selectors, `*`, `*?`, simple relationship patterns, layout, defaults, animation, titles, descriptions, and properties.
- Semantic validation for duplicate identifiers and view keys, references, parent hierarchy, filters, deployment environments, dynamic scopes, and view scope types.
- Element/relationship styles, light/dark variants, theme references, branding, and terminology with validation and safe offline preservation.
- Deterministic Mermaid class/link styling for supported colors, strokes, borders, thickness, line styles, and basic shapes.
- Safe local file/directory includes, constants, source-mapped substitution, and deterministic dependency tracking.
- Element/relationship selector expressions with tag, type, property, boolean, and comparison operators.
- Strict-safe validation and explicit rejection of remote includes, scripts, and plugins without execution.
- Safe local Markdown/AsciiDoc and ADR import attached to workspaces, software systems, and containers.
- Deterministic local static documentation sites with escaped HTML and raw Mermaid artifacts.
- Deterministic JSON, Mermaid, D2, PlantUML, C4-PlantUML, Graphviz DOT, Draw.io, ArchiMate, and HTML exporters.
- Archi-native `.archimate` export with editable diagram views.
- Optional SVG/PNG generation through an explicitly requested local Graphviz renderer.


Full Structurizr DSL support is planned incrementally; see ROADMAP.md.

## Milestone 3 additions

- Core workspace properties, local workspace extension, docs/ADR preservation, and configuration preservation.
- Flat/hierarchical identifiers, enterprise/groups, generic elements, and common child properties.
- Deployment environments, groups, nodes, infrastructure nodes, system/container instances, and health checks.
- Declaration-order reference validation and relationship removal with `-/>`.
- Safe rejection of remote extensions, scripts, plugins, and custom implied-relationship classes.

## Milestone 4 additions

- Full view header and child grammar for every M4 view type.
- Static, filtered, dynamic, and deployment view validation and deterministic expansion.
- Mermaid graph/sequence output while preserving M1 System Context bubbling.
- Safe local image metadata preservation and rejection of remote image/rendering URLs.

## Milestone 4.5 additions

- Committed Tree-sitter grammar and generated parser for all supported M1-M4 syntax.
- Tree-sitter-first parser facade with CST validation before existing semantic model construction.
- Minimal highlighting, folding, and locals queries for future editor support.
- Fixture-wide CST and semantic parity tests.

## Milestone 5 additions

- Tree-sitter and semantic support for styles, light/dark variants, themes, branding, and terminology.
- Validation for documented shapes, enums, booleans, integers, and numeric ranges.
- Mermaid element classes, basic shape approximation, relationship line styling, and terminology labels.
- Remote/local asset references remain metadata-only and are never fetched.

## Milestone 6 additions

- Local `!include` files and non-recursive directories with cycle detection and stable ordering.
- Ordered `!constant` definitions and `${NAME}` substitution in quoted and unquoted values.
- Source-segment mapping keeps included-file diagnostics attached to their original paths.
- Safe expression evaluation for view selectors and `--strict-safe` supply-chain validation.
- Scripts/plugins and remote includes are parsed but never executed or fetched.

## Milestone 7 additions

- Local `!docs` imports for Markdown directories/files and escaped AsciiDoc text.
- Local `!adrs` imports for adr-tools Markdown plus partial MADR/Log4brains metadata.
- `docs` static-site generation with local CSS, escaped content, and local `.mmd` diagrams.
- Deterministic `adr list` terminal output and strict-safe rejection of custom importers.

## Milestone 8 additions

- A deterministic exporter layer with stable filenames, ordering, IDs, and escaping.
- Structurizr-compatible JSON subset with c4c metadata for model, styles, views, docs, and ADRs.
- Per-view D2, generic PlantUML, local-macro C4-PlantUML, DOT, and importable Draw.io XML.
- ArchiMate 3.0 Model Exchange XML using a conservative C4 mapping and preserved c4c properties.
- `--format html` delegates to the M7 static site generator without changing `docs` behavior.
- SVG/PNG use only the local Graphviz `dot` executable and are rejected by `--strict-safe`.

## Milestone 8.1 additions

- Native Archi 5 XML with standard folders, mapped elements and relationships, and one editable diagram per c4c view.
- Deterministic diagram objects, grid bounds, and connections without executing Archi or another renderer.
- Separate `archi` aliases preserve the standards-oriented Open Group `archimate` exporter.
- Safe native import, canonical diff, C4 projection, and lossless sidecar round-tripping for unchanged projections.

## Build

```bash
cargo build --release
```

Normal Rust builds use committed Tree-sitter C artifacts and do not require
Node.js, npm, or grammar regeneration.

## Tree-sitter grammar development

[`tree-sitter-structurizr-dsl/grammar.js`](tree-sitter-structurizr-dsl/grammar.js)
is the syntax source of truth. Tree-sitter generates `src/parser.c`,
`src/grammar.json`, `src/node-types.json`, and the headers under
`src/tree_sitter/`. These artifacts are committed intentionally so Rust users
do not need Node.js.

Node.js is only needed when changing the grammar:

```bash
cd tree-sitter-structurizr-dsl
npm install
cd ..
make grammar
make grammar-test
```

After regeneration, verify both generated artifacts and Rust behavior:

```bash
make check
```

## Run

```bash
cargo run -- validate examples/internet-banking.dsl
cargo run -- inspect examples/internet-banking.dsl
cargo run -- export examples/internet-banking.dsl --format mermaid --out out
cargo run -- export examples/internet-banking.dsl --format json --out out-json
cargo run -- export examples/internet-banking.dsl --format d2 --out out-d2
cargo run -- export examples/internet-banking.dsl --format plantuml --out out-plantuml
cargo run -- export examples/internet-banking.dsl --format c4plantuml --out out-c4plantuml
cargo run -- export examples/internet-banking.dsl --format dot --out out-dot
cargo run -- export examples/internet-banking.dsl --format drawio --out out-drawio
cargo run -- export examples/internet-banking.dsl --format archimate --out out-archimate
cargo run -- export examples/internet-banking.dsl --format archi --out out-archi
cargo run -- export tests/fixtures/m7-docs.dsl --format html --out out-html
cargo run -- docs tests/fixtures/m7-docs.dsl --out site
cargo run -- adr list tests/fixtures/m7-docs.dsl
```

After export:

```text
out/system-context.mmd
out/container.mmd
```

## Example

```dsl
workspace "Internet Banking" "Milestone 1 example" {

  !identifiers hierarchical

  model {
    customer = person "Customer" "A bank customer"

    bank = softwareSystem "Internet Banking System" "Allows customers to view accounts and make payments" {
      web = container "Web Application" "Customer-facing SPA" "React"
      api = container "API Application" "Backend API" "Rust"
      db = container "Database" "Stores audit/profile data" "PostgreSQL"
    }

    customer -> bank.web "Uses"
    bank.web -> bank.api "Calls" "HTTPS/JSON"
    bank.api -> bank.db "Reads/writes" "SQL"
  }

  views {
    systemContext bank "system-context" "System Context diagram" {
      include *
      autolayout lr
    }

    container bank "container" "Container diagram" {
      include *
      autolayout lr
    }
  }
}
```

## Design

Compiler pipeline:

```text
workspace.dsl
  -> preprocessor
  -> Tree-sitter CST
  -> semantic-model adapter
  -> AST / workspace model
  -> semantic validator
  -> view model
  -> exporters
```

The compiler now has a Tree-sitter syntax frontend, safe preprocessing, span-aware source diagnostics, semantic validation, view expansion, styling, local documentation/ADR site generation, and a deterministic local exporter layer.

## Export formats

Text exports require no renderer or network access:

| Format | Output |
| --- | --- |
| `json`, `structurizr-json` | `workspace.json` |
| `mermaid`, `mmd` | `<view-key>.mmd` |
| `d2` | `<view-key>.d2` |
| `plantuml`, `puml` | `<view-key>.puml` |
| `c4plantuml`, `c4-plantuml` | `<view-key>.puml` |
| `dot`, `graphviz` | `<view-key>.dot` |
| `drawio`, `draw.io` | `<view-key>.drawio` |
| `archimate`, `archimate-xml`, `opengroup-archimate` | `workspace.archimate.xml` |
| `archi`, `archi-native`, `archimate-native` | `workspace.archimate` native Archi XML with editable diagrams |
| `html`, `site` | M7 static site |

The JSON output is a documented Structurizr-compatible subset; the `c4c` object preserves
the complete element kinds and local metadata. Draw.io uses a deterministic grid layout.
ArchiMate export is a pragmatic exchange mapping, not full semantic equivalence: people map
to `BusinessActor`, C4 software concepts to `ApplicationComponent`, deployment/infrastructure
nodes to `Node`, generic/deployment grouping concepts to `Grouping`, and relationships to the
conservative `Association` type. ArchiMate visual views are deferred.

`archimate` exports Open Group ArchiMate Model Exchange XML for standards-based interchange.
`archi` exports Archi's native, Archi-specific `.archimate` XML so Archi can open the model
directly with editable diagram views. Container scopes are native nested diagram objects and
connections are emitted only when both endpoint objects exist in the same view.

## Archi native round-trip

Native import generates a C4-compatible projection plus a JSON sidecar. The sidecar stores the
complete original native XML, preserving folder/view order, IDs, groups, bounds, colors, fonts,
connections, routing, and unknown native content. It is used only while the DSL file still matches
the imported projection; a changed projection falls back to deterministic native generation with a
warning instead of applying stale references.

The projection uses readable name-derived identifiers. It preserves model hierarchy, native
relationship types, view membership, and visual groups in ordinary DSL constructs and tags, so a
sidecar-free export remains logically equivalent. Exact native identity, colors, bounds, routing,
fonts, and unknown native content remain in the sidecar.

```bash
cargo run -- archi import model.archimate \
  --out workspace.dsl --sidecar workspace.archi-sidecar.json
cargo run -- export workspace.dsl --format archi --out out \
  --archi-sidecar workspace.archi-sidecar.json
cargo run -- archi diff model.archimate out/workspace.archimate
cargo run -- archi diff model.archimate out-without-sidecar/workspace.archimate --semantic
```

`archi diff` ignores insignificant XML whitespace and attribute ordering, preserves meaningful
child order and references, and treats `targetConnections` values as a set. The importer maps
`BusinessActor` to `person`, `ApplicationComponent` to `softwareSystem`, `Node` to
`deploymentNode`, and other native concepts to generic elements; exact native types remain in the
sidecar. `archi diff --semantic` ignores generated IDs and visual formatting while comparing model
folders, element and relationship semantics, view membership, groups, connections, and connection
integrity. Merging arbitrary DSL edits back into the native sidecar is intentionally deferred.

`svg` and `png` explicitly run the local Graphviz `dot` binary. If it is absent, c4c reports an
installation hint and does not contact a remote service. `--strict-safe` rejects all renderer
execution; use `dot`, `mermaid`, or another text format instead.

## Offline/security policy

The project should remain local-first:

- No telemetry.
- No network calls by default.
- No remote rendering.
- No cloud dependency.
- Text exporters never execute external binaries; SVG/PNG may execute only local Graphviz after an explicit request.
- Archi native export is text-only and remains available under `--strict-safe`.
- Remote `!include <url>` is rejected without making a request; `--allow-network` is parsed but fetching remains unimplemented.
- `--strict-safe` rejects remote assets and executable directives.
- Documentation imports are confined below the declaring DSL file and never load custom classes or remote content.

## License

MIT
