use scraper::{Html, Selector};

/// Extracts the visible text of every element matching `selector`, collapsing
/// whitespace. Returns an empty string if the selector is invalid or matches
/// nothing — callers treat that as "no body found" rather than a hard error,
/// since a single regulator changing their page markup shouldn't crash the
/// whole crawl run.
pub(crate) fn extract_text(html: &str, selector: &str) -> String {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse(selector) else {
        return String::new();
    };

    let raw = document
        .select(&selector)
        .flat_map(|el| el.text())
        .collect::<Vec<_>>()
        .join(" ");

    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_collapses_whitespace_from_matching_elements() {
        let html = "<html><body><div class=\"body\"> Hello   <b>world</b>\n\t</div></body></html>";
        assert_eq!(extract_text(html, "div.body"), "Hello world");
    }

    #[test]
    fn returns_empty_string_when_selector_matches_nothing() {
        let html = "<html><body><p>irrelevant</p></body></html>";
        assert_eq!(extract_text(html, "div.field--name-body"), "");
    }
}
