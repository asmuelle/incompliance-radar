//! Core domain models shared by the backend, the frontend (via WASM) and the LLM
//! extraction pipeline. Kept dependency-light (serde + chrono + uuid only) so it can
//! compile to wasm32-unknown-unknown for the Leptos client as well as native targets.

pub mod case;
pub mod company;
pub mod monitor;
pub mod regulator;
pub mod resolution;
pub mod sanction;
pub mod violation;

pub use case::ComplianceCase;
pub use company::{Company, CompanyId};
pub use monitor::Monitor;
pub use regulator::Regulator;
pub use resolution::{Resolution, ResolutionKind, ResolutionStatus};
pub use sanction::Sanction;
pub use violation::ViolationType;
