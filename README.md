# c4c — local Structurizr DSL compiler prototype

`c4c` is a Rust prototype for a local/offline Structurizr DSL compiler.

Goal: parse Structurizr-style C4 architecture-as-code, validate it locally, and export static artifacts without SaaS, remote rendering, accounts, or telemetry.

## Current scope

Implemented in this repo:

- CLI with `validate`, `inspect`, and `export` commands.
- Hand-written lexer/parser for a useful Structurizr DSL subset.
- Workspace parsing with optional name and description.
- `!identifiers hierarchical`.
- `model` block.
- `person`.
- `softwareSystem` with nested `container`.
- `container` with optional description and technology.
- `component` data model support in parser, although views/export are basic.
- Relationships using `->` with optional description and technology.
- `views` block.
- `systemContext` view.
- `container` view.
- `include *`, `exclude`, `autolayout`, and `title` inside supported views.
- Semantic validation for duplicate identifiers, relationship endpoints, parent hierarchy, and view scope types.
- Mermaid export for supported views.


Full Structurizr DSL support is planned incrementally; see ROADMAP.md.

## Milestone 3 additions

- Core workspace properties, local workspace extension, docs/ADR preservation, and configuration preservation.
- Flat/hierarchical identifiers, enterprise/groups, generic elements, and common child properties.
- Deployment environments, groups, nodes, infrastructure nodes, system/container instances, and health checks.
- Declaration-order reference validation and relationship removal with `-/>`.
- Safe rejection of remote extensions, scripts, plugins, and custom implied-relationship classes.

M4+ views, styles, preprocessing, documentation rendering, and additional exporters remain deferred.

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run -- validate examples/internet-banking.dsl
cargo run -- inspect examples/internet-banking.dsl
cargo run -- export examples/internet-banking.dsl --format mermaid --out out
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
  -> lexer
  -> parser
  -> AST / workspace model
  -> semantic validator
  -> view model
  -> exporters
```

The compiler now has a span-aware lexer/parser, source diagnostics, semantic validation, and core workspace/model grammar support. Full view grammar, styles, preprocessing, documentation rendering, and additional exporters are planned in later milestones.

## Offline/security policy

The project should remain local-first:

- No telemetry.
- No network calls by default.
- No remote rendering.
- No cloud dependency.
- Remote `!include <url>` should require an explicit opt-in flag if implemented.

## License

MIT
