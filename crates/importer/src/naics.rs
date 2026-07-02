//! Maps 6-digit NAICS codes (as exported by the registry) to the standard
//! 2-digit NAICS sector names, so `Company.industry` holds a human-readable
//! label consistent across the imported corpus instead of a numeric code.

/// Sector name for a raw NAICS code string, or `None` if the code is empty
/// or doesn't start with a known 2-digit sector prefix.
pub fn sector_name(naics: &str) -> Option<&'static str> {
    let prefix: u32 = naics.trim().get(..2)?.parse().ok()?;
    let name = match prefix {
        11 => "Agriculture, Forestry, Fishing & Hunting",
        21 => "Mining, Quarrying, Oil & Gas Extraction",
        22 => "Utilities",
        23 => "Construction",
        31..=33 => "Manufacturing",
        42 => "Wholesale Trade",
        44 | 45 => "Retail Trade",
        48 | 49 => "Transportation & Warehousing",
        51 => "Information",
        52 => "Finance & Insurance",
        53 => "Real Estate, Rental & Leasing",
        54 => "Professional, Scientific & Technical Services",
        55 => "Management of Companies & Enterprises",
        56 => "Administrative Support & Waste Management",
        61 => "Educational Services",
        62 => "Health Care & Social Assistance",
        71 => "Arts, Entertainment & Recreation",
        72 => "Accommodation & Food Services",
        81 => "Other Services",
        92 => "Public Administration",
        _ => return None,
    };
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_six_digit_code_to_sector() {
        assert_eq!(sector_name("446110"), Some("Retail Trade"));
        assert_eq!(sector_name("331513"), Some("Manufacturing"));
        assert_eq!(sector_name("522110"), Some("Finance & Insurance"));
    }

    #[test]
    fn unknown_or_empty_codes_map_to_none() {
        assert_eq!(sector_name(""), None);
        assert_eq!(sector_name("99"), None);
        assert_eq!(sector_name("x"), None);
    }
}
