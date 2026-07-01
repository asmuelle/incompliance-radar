CREATE TABLE watch_rules (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    industry TEXT,
    company_name_contains TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- watch_rule_label/company_name are copied at write time (see domain::Alert)
-- rather than joined at read time, so the alert feed stays readable even if
-- the rule or case it references is later deleted; no foreign keys.
CREATE TABLE alerts (
    id TEXT PRIMARY KEY,
    watch_rule_id TEXT NOT NULL,
    watch_rule_label TEXT NOT NULL,
    case_id TEXT NOT NULL,
    company_name TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    acknowledged INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_alerts_created_at ON alerts (created_at);
