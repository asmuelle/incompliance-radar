//! Core domain models shared by the backend, the frontend (via WASM) and the LLM
//! extraction pipeline. Kept dependency-light (serde + chrono + uuid only) so it can
//! compile to wasm32-unknown-unknown for the Leptos client as well as native targets.

pub mod alert;
pub mod case;
pub mod company;
pub mod monitor;
pub mod regime;
pub mod regulator;
pub mod resolution;
pub mod sanction;
pub mod trend_report;
pub mod violation;
pub mod watch_rule;

pub use alert::Alert;
pub use case::ComplianceCase;
pub use company::{Company, CompanyId};
pub use monitor::Monitor;
pub use regime::Regime;
pub use regulator::Regulator;
pub use resolution::{Resolution, ResolutionKind, ResolutionStatus};
pub use sanction::Sanction;
pub use trend_report::{compute_trend_report, AmountEntry, CountEntry, RateEntry, TrendReport};
pub use violation::ViolationType;
pub use watch_rule::WatchRule;
