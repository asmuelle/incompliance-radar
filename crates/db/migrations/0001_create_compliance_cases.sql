CREATE TABLE compliance_cases (
    id TEXT PRIMARY KEY,
    company_name TEXT NOT NULL,
    industry TEXT NOT NULL,
    jurisdiction TEXT NOT NULL,
    -- Full serialized domain::ComplianceCase (resolutions, monitors, sanctions, ...).
    -- Indexed columns above cover today's filtering needs; normalize further only
    -- once real query patterns (search/filter UI, roadmap item 4) demand it.
    data TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_compliance_cases_industry ON compliance_cases (industry);
CREATE INDEX idx_compliance_cases_jurisdiction ON compliance_cases (jurisdiction);
