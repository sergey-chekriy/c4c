# ArchiMate 3.2 registry audit

Audit source: `~/Downloads/archimate_3_2_reference_cards.pdf`, extracted locally with `pdftotext`.

Result: every current c4c M8.7 ArchiMate element and relationship is present in the ArchiMate 3.2 reference cards. No implementation-impacting removals, renames, or additions were identified from the available cards.

| Kind | c4c name | ArchiMate 3.2 status | Canonical ArchiMate 3.2 name | Compatibility policy | Tests |
| --- | --- | --- | --- | --- | --- |
| Elements | Motivation set | present | Stakeholder, Driver, Assessment, Goal, Outcome, Principle, Requirement, Constraint, Meaning, Value | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Strategy set | present | Resource, Capability, Value Stream, Course of Action | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Business set | present | Business Actor, Business Role, Business Collaboration, Business Interface, Business Process, Business Function, Business Interaction, Business Event, Business Service, Business Object, Contract, Representation, Product | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Application set | present | Application Component, Application Collaboration, Application Interface, Application Function, Application Interaction, Application Process, Application Event, Application Service, Data Object | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Technology set | present | Node, Device, System Software, Technology Collaboration, Technology Interface, Path, Communication Network, Technology Function, Technology Process, Technology Interaction, Technology Event, Technology Service, Artifact | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Physical set | present | Equipment, Facility, Distribution Network, Material | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Implementation/Migration set | present | Work Package, Deliverable, Implementation Event, Plateau, Gap | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Elements | Composite set | present | Grouping, Location | keep existing keywords/native/Open Group names | `m88-archimate-32-full-vocabulary.dsl` |
| Connectors | junction/andJunction/orJunction | present | Junction | keep current explicit connector keywords | compiler registry tests |
| Relationships | structural/dependency/dynamic/other set | present | Composition, Aggregation, Assignment, Realization, Serving, Access, Influence, Association, Triggering, Flow, Specialization | keep current relationship type aliases and native/Open Group mappings | compiler/exporter tests |
| Viewpoints | supported c4c metadata names | uncertain | not listed in extracted reference cards | keep M8.7 supported metadata; deeper audit needs full spec | compiler validation tests |

Native Archi mappings remain Archi-tool-specific and are not derived from the reference cards. Open Group export continues to use the existing ArchiMate 3.x exchange namespace and type names pending full 3.2 specification review.
