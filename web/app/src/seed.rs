//! In-memory demo data so the app runs out of the box with no database.
//! Replace with a real repository (see `docs/architecture.md`) once persistence lands.

use domain::{
    Company, ComplianceCase, Monitor, Regulator, Resolution, ResolutionKind, ResolutionStatus,
    Sanction, ViolationType,
};

pub fn seed_cases() -> Vec<ComplianceCase> {
    vec![fictional_bribery_case(), fictional_sanctions_case()]
}

fn fictional_bribery_case() -> ComplianceCase {
    let company = Company::new(
        "Acme Global Industries (fictional)",
        "Industrial Manufacturing",
        "US",
    );
    let mut case = ComplianceCase::new(company);
    case.resolutions.push(Resolution {
        regulator: Regulator::Doj,
        kind: ResolutionKind::DeferredProsecutionAgreement,
        status: ResolutionStatus::Active,
        signed_on: chrono::NaiveDate::from_ymd_opt(2024, 3, 15),
        term_months: Some(36),
        monitor: Some(Monitor {
            name: "Jane Doe (fictional)".into(),
            firm: Some("Example Compliance Advisors LLP".into()),
            appointed_on: chrono::NaiveDate::from_ymd_opt(2024, 4, 1),
            term_months: Some(36),
        }),
        violations: vec![ViolationType::Bribery],
        sanctions: vec![Sanction {
            amount: 45_000_000.0,
            currency: "USD".into(),
            description: Some("Criminal penalty".into()),
        }],
        obligations: vec![
            "Enhance third-party due diligence procedures".into(),
            "Report to DoJ quarterly".into(),
        ],
        source: None,
    });
    case
}

fn fictional_sanctions_case() -> ComplianceCase {
    let company = Company::new("Northbridge Financial Group (fictional)", "Banking", "UK");
    let mut case = ComplianceCase::new(company);
    case.resolutions.push(Resolution {
        regulator: Regulator::Ofac,
        kind: ResolutionKind::ConsentOrder,
        status: ResolutionStatus::Completed,
        signed_on: chrono::NaiveDate::from_ymd_opt(2022, 9, 1),
        term_months: Some(24),
        monitor: None,
        violations: vec![
            ViolationType::SanctionsViolation,
            ViolationType::MoneyLaundering,
        ],
        sanctions: vec![Sanction {
            amount: 12_500_000.0,
            currency: "USD".into(),
            description: Some("Civil monetary penalty".into()),
        }],
        obligations: vec!["Overhaul sanctions screening system".into()],
        source: None,
    });
    case
}
