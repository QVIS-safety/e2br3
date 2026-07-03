//! Presave section REST handlers, split by entity group.
//!
//! Each submodule owns one entity family (CRUD + detail handlers). Shared
//! scope/permission helpers live in `shared`. Handlers are re-exported so the
//! router paths (`section_presave_rest::<handler>`) stay unchanged.

mod shared;

mod narrative;
mod product;
mod receiver;
mod reporter;
mod sender;
mod study;

pub use narrative::*;
pub use product::*;
pub use receiver::*;
pub use reporter::*;
pub use sender::*;
pub use study::*;
