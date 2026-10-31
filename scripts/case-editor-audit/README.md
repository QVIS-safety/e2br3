# Persisted-field execution hints

`case-edit-persisted-field-inventory.json` retains historical field descriptors
used by `registry/tools/build_ui_inventory.py --execution-hints`. CI reconciles
these descriptors with registry mappings, frontend bindings and editor contracts;
unmapped entries fail the gate. The canonical field denominator comes from
`registry/`, not the historical descriptor count.

The standalone runners and alternate-value map have been removed. Historical
counts, inclusion flags, source paths and resolution evidence in the JSON are
provenance only; they are not current test results. Current execution evidence
comes from the linked frontend contract and backend API tests.
