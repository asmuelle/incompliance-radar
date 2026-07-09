use serde::{Deserialize, Serialize};
use std::fmt;

use crate::Regime;

/// Regulatory or enforcement body that brought or oversees a resolution.
///
/// A struct backed by a curated static registry ([`REGISTRY`]) rather than an
/// enum: five variants worked when coverage was DoJ/SEC/FCA/OFAC/SFO, but a
/// multi-regime EnforcementRadar needs ~30 EU data-protection authorities and
/// dozens of US federal/state bodies, and an enum would push every one of
/// them through `Other(String)` — losing exactly the jurisdiction/regime
/// structure this struct keeps.
///
/// Serialized as a plain struct map. Deserialization additionally accepts the
/// legacy enum encodings still present in stored case JSON (`"Doj"`, `"Sec"`,
/// ..., and `{"Other": "..."}`) — see the custom `Deserialize` impl and its
/// tests. Don't remove that path without migrating the `compliance_cases`
/// table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Regulator {
    /// Canonical machine identifier, e.g. `"us-doj"` — what watch rules and
    /// (later) search filters match on.
    pub slug: String,
    /// Full official name, e.g. "US Department of Justice".
    pub name: String,
    /// Compact label for UI display, e.g. "DoJ". Falls back to `name` for
    /// regulators outside the registry.
    pub short_name: String,
    /// ISO-ish short code as used elsewhere in the app ("US", "UK", "EU",
    /// "IE", ...). `None` when unknown (e.g. free text from extraction).
    pub jurisdiction: Option<String>,
    /// The regulator's *primary* enforcement regime — used as the default
    /// regime for a resolution when the filing itself doesn't pin one down.
    /// `None` when unknown.
    pub regime: Option<Regime>,
}

/// One curated registry entry. `&'static` fields keep the table a plain
/// `const` — [`Regulator`]s are materialized from it on demand.
struct RegulatorSpec {
    slug: &'static str,
    name: &'static str,
    short_name: &'static str,
    jurisdiction: &'static str,
    regime: Regime,
    /// Lowercase alternative names/spellings matched by [`Regulator::normalize`],
    /// in addition to slug/short_name/name.
    aliases: &'static [&'static str],
}

/// Curated, deliberately small: an entry earns its place by having a
/// connector, an importer, or extraction output that actually names it.
/// Growing it is additive — no call site enumerates the registry.
const REGISTRY: &[RegulatorSpec] = &[
    RegulatorSpec {
        slug: "us-doj",
        name: "US Department of Justice",
        short_name: "DoJ",
        jurisdiction: "US",
        regime: Regime::CorporateProsecution,
        aliases: &[
            "department of justice",
            "justice department",
            "u.s. department of justice",
        ],
    },
    RegulatorSpec {
        slug: "us-sec",
        name: "US Securities and Exchange Commission",
        short_name: "SEC",
        jurisdiction: "US",
        regime: Regime::SecuritiesEnforcement,
        aliases: &["securities and exchange commission"],
    },
    RegulatorSpec {
        slug: "uk-fca",
        name: "UK Financial Conduct Authority",
        short_name: "FCA",
        jurisdiction: "UK",
        regime: Regime::SecuritiesEnforcement,
        aliases: &["financial conduct authority"],
    },
    RegulatorSpec {
        slug: "us-ofac",
        name: "US Office of Foreign Assets Control",
        short_name: "OFAC",
        jurisdiction: "US",
        regime: Regime::SanctionsEnforcement,
        aliases: &["office of foreign assets control"],
    },
    RegulatorSpec {
        slug: "uk-sfo",
        name: "UK Serious Fraud Office",
        short_name: "SFO",
        jurisdiction: "UK",
        regime: Regime::CorporateProsecution,
        aliases: &["serious fraud office"],
    },
    RegulatorSpec {
        slug: "us-fed",
        name: "Board of Governors of the Federal Reserve System",
        short_name: "Federal Reserve",
        jurisdiction: "US",
        regime: Regime::BankingSupervision,
        aliases: &["federal reserve board", "frb", "the fed", "fed"],
    },
    RegulatorSpec {
        slug: "us-occ",
        name: "Office of the Comptroller of the Currency",
        short_name: "OCC",
        jurisdiction: "US",
        regime: Regime::BankingSupervision,
        aliases: &["comptroller of the currency"],
    },
    RegulatorSpec {
        slug: "us-fdic",
        name: "Federal Deposit Insurance Corporation",
        short_name: "FDIC",
        jurisdiction: "US",
        regime: Regime::BankingSupervision,
        aliases: &[],
    },
    RegulatorSpec {
        slug: "us-ftc",
        name: "Federal Trade Commission",
        short_name: "FTC",
        jurisdiction: "US",
        regime: Regime::ConsumerProtection,
        aliases: &[],
    },
    RegulatorSpec {
        slug: "us-cfpb",
        name: "Consumer Financial Protection Bureau",
        short_name: "CFPB",
        jurisdiction: "US",
        regime: Regime::ConsumerProtection,
        aliases: &[],
    },
    RegulatorSpec {
        slug: "ie-dpc",
        name: "Data Protection Commission (Ireland)",
        short_name: "DPC",
        jurisdiction: "IE",
        regime: Regime::DataProtection,
        aliases: &[
            "data protection commission",
            "irish data protection commission",
            "an coimisiún um chosaint sonraí",
        ],
    },
    RegulatorSpec {
        slug: "uk-ico",
        name: "UK Information Commissioner's Office",
        short_name: "ICO",
        jurisdiction: "UK",
        regime: Regime::DataProtection,
        aliases: &[
            "information commissioner's office",
            "information commissioners office",
        ],
    },
    RegulatorSpec {
        slug: "fr-cnil",
        name: "Commission Nationale de l'Informatique et des Libertés",
        short_name: "CNIL",
        jurisdiction: "FR",
        regime: Regime::DataProtection,
        aliases: &["commission nationale de l'informatique et des libertes"],
    },
    RegulatorSpec {
        slug: "eu-edpb",
        name: "European Data Protection Board",
        short_name: "EDPB",
        jurisdiction: "EU",
        regime: Regime::DataProtection,
        aliases: &[],
    },
];

impl Regulator {
    fn from_spec(spec: &RegulatorSpec) -> Self {
        Self {
            slug: spec.slug.to_string(),
            name: spec.name.to_string(),
            short_name: spec.short_name.to_string(),
            jurisdiction: Some(spec.jurisdiction.to_string()),
            regime: Some(spec.regime.clone()),
        }
    }

    /// Looks up a registry entry by its canonical slug (case-insensitive).
    pub fn from_slug(slug: &str) -> Option<Self> {
        REGISTRY
            .iter()
            .find(|spec| spec.slug.eq_ignore_ascii_case(slug.trim()))
            .map(Self::from_spec)
    }

    /// Maps free text — extraction output, importer columns, user input —
    /// onto a registry entry by slug, short name, full name, or alias
    /// (case-insensitive). Text that matches nothing becomes an [`other`]
    /// regulator so no information is dropped at the boundary.
    ///
    /// [`other`]: Regulator::other
    pub fn normalize(value: &str) -> Self {
        let wanted = value.trim().to_lowercase();
        REGISTRY
            .iter()
            .find(|spec| {
                spec.slug == wanted
                    || spec.short_name.to_lowercase() == wanted
                    || spec.name.to_lowercase() == wanted
                    || spec.aliases.contains(&wanted.as_str())
            })
            .map(Self::from_spec)
            .unwrap_or_else(|| Self::other(value))
    }

    /// A regulator outside the curated registry, carried verbatim (with a
    /// derived slug) rather than force-fitted — same role `Regulator::Other`
    /// played in the old enum.
    pub fn other(name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        Self {
            slug: derive_slug(&name),
            short_name: name.clone(),
            name,
            jurisdiction: None,
            regime: None,
        }
    }

    pub fn doj() -> Self {
        Self::from_spec(&REGISTRY[0])
    }

    pub fn sec() -> Self {
        Self::from_spec(&REGISTRY[1])
    }

    pub fn fca() -> Self {
        Self::from_spec(&REGISTRY[2])
    }

    pub fn ofac() -> Self {
        Self::from_spec(&REGISTRY[3])
    }

    pub fn sfo() -> Self {
        Self::from_spec(&REGISTRY[4])
    }
}

/// Kebab-cases a free-text name into a stable slug: `"BaFin"` → `"bafin"`,
/// `"NY DFS"` → `"ny-dfs"`.
fn derive_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_dash = true; // suppress leading dashes
    for c in name.chars() {
        if c.is_alphanumeric() {
            slug.extend(c.to_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

impl fmt::Display for Regulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short_name)
    }
}

/// Accepts three shapes: the current struct map, a legacy enum unit-variant
/// string (`"Doj"`, `"Sec"`, `"Fca"`, `"Ofac"`, `"Sfo"`), and the legacy
/// externally-tagged `{"Other": "name"}` map. The legacy shapes are what
/// every case persisted before the registry refactor contains — SQLite rows
/// store the full `ComplianceCase` as JSON, so old encodings live in the
/// database indefinitely.
impl<'de> Deserialize<'de> for Regulator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            LegacyOther {
                #[serde(rename = "Other")]
                name: String,
            },
            Full {
                slug: String,
                name: String,
                short_name: String,
                #[serde(default)]
                jurisdiction: Option<String>,
                #[serde(default)]
                regime: Option<Regime>,
            },
            LegacyTag(String),
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::LegacyOther { name } => Regulator::other(name),
            Repr::Full {
                slug,
                name,
                short_name,
                jurisdiction,
                regime,
            } => Regulator {
                slug,
                name,
                short_name,
                jurisdiction,
                regime,
            },
            Repr::LegacyTag(tag) => match tag.as_str() {
                "Doj" => Regulator::doj(),
                "Sec" => Regulator::sec(),
                "Fca" => Regulator::fca(),
                "Ofac" => Regulator::ofac(),
                "Sfo" => Regulator::sfo(),
                other => Regulator::normalize(other),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_convenience_constructors_match_from_slug() {
        assert_eq!(Regulator::doj(), Regulator::from_slug("us-doj").unwrap());
        assert_eq!(Regulator::sec(), Regulator::from_slug("us-sec").unwrap());
        assert_eq!(Regulator::fca(), Regulator::from_slug("uk-fca").unwrap());
        assert_eq!(Regulator::ofac(), Regulator::from_slug("us-ofac").unwrap());
        assert_eq!(Regulator::sfo(), Regulator::from_slug("uk-sfo").unwrap());
    }

    #[test]
    fn registry_slugs_are_unique() {
        let mut slugs: Vec<&str> = REGISTRY.iter().map(|s| s.slug).collect();
        slugs.sort_unstable();
        let before = slugs.len();
        slugs.dedup();
        assert_eq!(before, slugs.len());
    }

    #[test]
    fn normalize_matches_short_name_case_insensitively() {
        assert_eq!(Regulator::normalize("doj"), Regulator::doj());
        assert_eq!(Regulator::normalize("SEC"), Regulator::sec());
        assert_eq!(Regulator::normalize("FDIC").slug, "us-fdic");
    }

    #[test]
    fn normalize_matches_full_names_and_aliases() {
        assert_eq!(
            Regulator::normalize("US Department of Justice"),
            Regulator::doj()
        );
        assert_eq!(Regulator::normalize("Justice Department"), Regulator::doj());
        assert_eq!(
            Regulator::normalize("Irish Data Protection Commission").slug,
            "ie-dpc"
        );
    }

    #[test]
    fn normalize_falls_back_to_other() {
        let bafin = Regulator::normalize("BaFin");
        assert_eq!(bafin.slug, "bafin");
        assert_eq!(bafin.name, "BaFin");
        assert_eq!(bafin.jurisdiction, None);
        assert_eq!(bafin.regime, None);
    }

    #[test]
    fn other_derives_kebab_slug() {
        assert_eq!(Regulator::other("NY DFS").slug, "ny-dfs");
        assert_eq!(Regulator::other("  Garante  ").name, "Garante");
    }

    #[test]
    fn registry_regulators_carry_regime_and_jurisdiction() {
        let dpc = Regulator::from_slug("ie-dpc").unwrap();
        assert_eq!(dpc.regime, Some(Regime::DataProtection));
        assert_eq!(dpc.jurisdiction.as_deref(), Some("IE"));
    }

    #[test]
    fn deserializes_legacy_unit_variant_strings() {
        // The exact encodings the old `enum Regulator` produced — present in
        // every case row persisted before the registry refactor.
        for (legacy, expected) in [
            ("\"Doj\"", Regulator::doj()),
            ("\"Sec\"", Regulator::sec()),
            ("\"Fca\"", Regulator::fca()),
            ("\"Ofac\"", Regulator::ofac()),
            ("\"Sfo\"", Regulator::sfo()),
        ] {
            let parsed: Regulator = serde_json::from_str(legacy).unwrap();
            assert_eq!(parsed, expected, "legacy tag {legacy}");
        }
    }

    #[test]
    fn deserializes_legacy_other_map() {
        let parsed: Regulator = serde_json::from_str(r#"{"Other": "BaFin"}"#).unwrap();
        assert_eq!(parsed, Regulator::other("BaFin"));
    }

    #[test]
    fn current_format_roundtrips() {
        let original = Regulator::from_slug("us-fed").unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Regulator = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn other_regulator_roundtrips() {
        let original = Regulator::other("Bavarian DPA");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Regulator = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }
}
