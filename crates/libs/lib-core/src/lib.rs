pub mod config;
pub mod ctx;
pub mod e2b;
pub mod model;
pub mod narrative_template;
pub mod regulatory;
pub mod report_due;
pub mod serde;
pub mod validation_report;
pub mod xml;

// #[cfg(test)] // Commented during early development.
pub mod _dev_utils;

use config::core_config;
