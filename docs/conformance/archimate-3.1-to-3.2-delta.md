# ArchiMate 3.1 to 3.2 delta

Local references:

- primary: `~/Downloads/archimate_3_2_reference_cards.pdf`
- optional older reference: `~/Downloads/archimate_3_1_spec.pdf`

Review method:

- extracted the 3.2 reference-card text locally with `pdftotext`
- compared visible element and relationship names against the existing M8.7 registry and fixtures
- did not fetch specs, schemas, or web pages

Summary:

No implementation-impacting 3.1 -> 3.2 differences were identified from the available reference cards. Full-spec-only details remain out of scope for M8.8.

Visible differences from the reference cards:

- relationship names remain Composition, Aggregation, Assignment, Realization, Serving, Access, Influence, Association, Triggering, Flow, Specialization, and Junction connectors
- element vocabulary visible in the cards matches the existing registry
- several definitions have wording changes, but no verified DSL/native/Open Group type-name change
- viewpoint details are not present in the extracted reference-card text

Implementation impact:

- mark c4c baseline as practical ArchiMate 3.2 support
- keep 3.1-compatible DSL keywords and aliases
- keep native Archi and Open Group type mappings unchanged
- add docs/tests proving the 3.2 audit result

Future:

- ArchiMate 4 is out of scope for M8.8 and tracked as future roadmap work
