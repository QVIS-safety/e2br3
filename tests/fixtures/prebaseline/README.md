# Historical test inputs

These SQL fragments exercise isolated old-schema conversion and authorization
reconciliation tests. They are not application migrations, are not shipped in
the runtime image, and are never loaded by bootstrap. Production schema is
only `db/bootstrap/*.sql`.
