use crate::{Company, Resolution};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A tracked compliance case: a company plus the history of resolutions
/// (DPAs, NPAs, monitorships) it has been subject to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceCase {
    pub id: Uuid,
    pub company: Company,
    pub resolutions: Vec<Resolution>,
}

impl ComplianceCase {
    pub fn new(company: Company) -> Self {
        Self {
            id: Uuid::new_v4(),
            company,
            resolutions: Vec::new(),
        }
    }

    pub fn active_resolutions(&self) -> impl Iterator<Item = &Resolution> {
        self.resolutions
            .iter()
            .filter(|r| r.status == crate::ResolutionStatus::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Regulator, ResolutionKind, ResolutionStatus};

    fn resolution_with_status(status: ResolutionStatus) -> Resolution {
        Resolution {
            regulator: Regulator::Doj,
            kind: ResolutionKind::DeferredProsecutionAgreement,
            status,
            signed_on: None,
            term_months: None,
            monitor: None,
            violations: Vec::new(),
            sanctions: Vec::new(),
            obligations: Vec::new(),
            source: None,
        }
    }

    #[test]
    fn active_resolutions_excludes_completed_and_terminated() {
        let mut case = ComplianceCase::new(Company::new("Acme", "Manufacturing", "US"));
        case.resolutions
            .push(resolution_with_status(ResolutionStatus::Active));
        case.resolutions
            .push(resolution_with_status(ResolutionStatus::Completed));
        case.resolutions
            .push(resolution_with_status(ResolutionStatus::Terminated));

        let active: Vec<_> = case.active_resolutions().collect();

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].status, ResolutionStatus::Active);
    }

    #[test]
    fn new_case_has_no_resolutions() {
        let case = ComplianceCase::new(Company::new("Acme", "Manufacturing", "US"));
        assert!(case.resolutions.is_empty());
        assert_eq!(case.active_resolutions().count(), 0);
    }
}
