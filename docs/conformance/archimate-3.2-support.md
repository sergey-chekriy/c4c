# ArchiMate 3.2 support

c4c provides practical ArchiMate 3.2 support. This is not an Open Group certification claim.

Baseline:

- ArchiMate language baseline: 3.2
- Support level: practical implementation
- Certification: not Open Group certified
- Source reviewed for M8.8: `~/Downloads/archimate_3_2_reference_cards.pdf`

Supported:

- explicit ArchiMate DSL vocabulary for Motivation, Strategy, Business, Application, Technology, Physical, Implementation & Migration, Composite, and Junction concepts visible in the 3.2 reference cards
- Composition, Aggregation, Assignment, Realization, Serving, Access, Influence, Association, Triggering, Flow, and Specialization relationships
- AccessRelationship direction metadata
- practical semantic warnings by default and strict semantic errors with `--strict` / `--strict-safe`
- native Archi `.archimate` import/export/check/diff
- Open Group ArchiMate exchange XML export
- sidecar round-trip for unchanged native projections
- deterministic no-sidecar native generation

Safety:

- c4c does not execute Archi, Java, renderers, scripts, or plugins for ArchiMate support
- c4c does not fetch schemas/specs or resolve external XML entities
- native export avoids unsupported `lineStyle` and semantic-element `fillColor`

Limitations:

- practical conformance only; not official certification
- viewpoint audit is limited by the reference-card scope
- Open Group exchange import is out of scope
- full edited-DSL sidecar merge is deferred
- ArchiMate 4 readiness is future roadmap work
