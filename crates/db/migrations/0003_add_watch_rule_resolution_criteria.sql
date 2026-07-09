-- Resolution-level watch-rule criteria (multi-regime EnforcementRadar,
-- phase 0). regime and violation_type store the domain enum as serde JSON
-- (e.g. '"DataProtection"' or '{"Other":"..."}') so Other(..) values
-- round-trip exactly; regulator_slug is the plain canonical slug
-- (domain::Regulator::slug). NULL on all three means "criterion not set" —
-- existing rows keep their meaning unchanged.
ALTER TABLE watch_rules ADD COLUMN regime TEXT;
ALTER TABLE watch_rules ADD COLUMN regulator_slug TEXT;
ALTER TABLE watch_rules ADD COLUMN violation_type TEXT;
