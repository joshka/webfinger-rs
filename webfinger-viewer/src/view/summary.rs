//! Parsed JSON Resource Descriptor display model.
//!
//! WebFinger responses are often malformed, partial, or implementation-specific when this viewer is
//! used for debugging. This module turns best-effort JSON parsing into stable template rows while
//! preserving raw output for anything the summary intentionally omits.

use serde_json::Value;
use url::Url;

use crate::lookup::LookupResult;

/// Structured summary of the parsed JSON Resource Descriptor.
///
/// Invalid JSON and sparse JSON are valid debugging outcomes, so this view model carries explicit
/// empty-state text instead of forcing the template to infer why there are no rows.
pub struct SummaryView {
    /// Subject, alias, and property rows rendered before links.
    pub rows: Vec<ResourceRow>,

    /// True when `rows` has at least one displayable resource field.
    pub has_rows: bool,

    /// JRD link rows rendered as a debugging table.
    pub links: Vec<LinkRow>,

    /// True when `links` has at least one displayable link row.
    pub has_links: bool,

    /// True when at least one link has fields outside rel/type/href/template.
    pub has_extra_fields: bool,

    /// Message shown when the response is not a parsed JRD or contains no displayable fields.
    pub empty_message: &'static str,

    /// True when the summary should show `empty_message` instead of rows.
    pub has_empty_message: bool,
}

impl SummaryView {
    /// Builds the displayable JRD summary from parsed JSON.
    ///
    /// The viewer is a debugging tool, so invalid JSON is not treated as a rendering panic or a
    /// blank section. It becomes an explicit empty state while raw body remains available below.
    pub fn from_json(json: Option<&Value>) -> Self {
        let Some(Value::Object(object)) = json else {
            return Self::empty("No JSON Resource Descriptor was parsed.");
        };

        let rows = resource_rows(object);
        let links_value = object.get("links").and_then(Value::as_array);
        let links = link_rows(links_value);
        let has_extra_fields = links.iter().any(|link| link.has_extra);
        let empty_message = if rows.is_empty() && links.is_empty() {
            "No JSON Resource Descriptor fields were found."
        } else {
            ""
        };
        let has_empty_message = !empty_message.is_empty();

        Self {
            has_rows: !rows.is_empty(),
            has_links: !links.is_empty(),
            rows,
            links,
            has_extra_fields,
            empty_message,
            has_empty_message,
        }
    }

    /// Builds a summary empty state while preserving the rest of the result panel.
    ///
    /// This keeps malformed or sparse responses inspectable: metadata, curl, and raw body still
    /// render even when there are no structured JRD rows to show.
    fn empty(message: &'static str) -> Self {
        Self {
            rows: Vec::new(),
            has_rows: false,
            links: Vec::new(),
            has_links: false,
            has_extra_fields: false,
            empty_message: message,
            has_empty_message: true,
        }
    }
}

/// One top-level JRD resource field rendered before the links table.
pub struct ResourceRow {
    /// Display name for the JRD field.
    pub key: String,

    /// Display value for the JRD field.
    pub value: String,
}

/// One JRD link object rendered in the links table.
pub struct LinkRow {
    /// JRD `rel`, or `(missing rel)` when the target omits it.
    pub rel: String,

    /// True when the link has either an `href` or `template` target.
    pub has_target: bool,

    /// Either `href`, `template`, or an empty string when the link has no target field.
    pub target_label: &'static str,

    /// Target field value displayed in the table.
    pub target_value: String,

    /// True when `target_value` should be rendered as a concrete anchor.
    pub has_target_href: bool,

    /// Clickable URL for `href` targets that use HTTP(S). Templates intentionally stay plain text.
    pub target_href: String,

    /// True when the target supplied a JRD link `type`.
    pub has_type: bool,

    /// Optional JRD link `type`.
    pub type_value: String,

    /// True when the link has fields outside rel/type/href/template.
    pub has_extra: bool,

    /// Extra link fields rendered as `key: value` lines.
    pub extra: String,
}

/// Converts top-level JRD subject, aliases, and properties into display rows.
///
/// These fields describe the resource rather than a specific relation. They are flattened into a
/// small key/value table so repeated aliases and arbitrary properties stay easy to scan.
fn resource_rows(object: &serde_json::Map<String, Value>) -> Vec<ResourceRow> {
    let mut rows = Vec::new();
    if let Some(subject) = object.get("subject").and_then(Value::as_str) {
        rows.push(ResourceRow {
            key: "subject".to_string(),
            value: subject.to_string(),
        });
    }
    if let Some(aliases) = object.get("aliases").and_then(Value::as_array) {
        for alias in aliases {
            rows.push(ResourceRow {
                key: "alias".to_string(),
                value: string_value(alias),
            });
        }
    }
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (key, value) in properties {
            rows.push(ResourceRow {
                key: key.clone(),
                value: string_value(value),
            });
        }
    }
    rows
}

/// Converts JRD `links` into display rows, ignoring malformed non-object entries.
///
/// RFC-shaped JRD links are objects. Skipping malformed entries keeps the UI stable for bad target
/// responses, and the raw JSON section remains the validation surface for seeing exactly what was
/// omitted from the summary.
fn link_rows(links: Option<&Vec<Value>>) -> Vec<LinkRow> {
    let Some(links) = links else {
        return Vec::new();
    };

    links.iter().filter_map(link_row).collect()
}

/// Converts one JRD link object into a row for the link table.
///
/// `rel` is the primary thing a WebFinger user is inspecting, so it becomes the row label. `href`
/// and `template` are mutually presented as the target column because one is a concrete URL and the
/// other is a URI template; mixing both into a generic key/value block made the result harder to
/// compare across links.
fn link_row(link: &Value) -> Option<LinkRow> {
    let object = link.as_object()?;
    let rel = object
        .get("rel")
        .and_then(Value::as_str)
        .unwrap_or("(missing rel)")
        .to_string();
    let target = LinkTarget::from_object(object);
    let type_value = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let extra = extra_fields(object);

    Some(LinkRow {
        rel,
        has_target: target.has_value,
        target_label: target.label,
        target_value: target.value,
        has_target_href: target.has_href,
        target_href: target.href,
        has_type: !type_value.is_empty(),
        type_value,
        has_extra: !extra.is_empty(),
        extra,
    })
}

/// Target column data for one JRD link.
///
/// Keeping this conversion in Rust keeps the template from nesting `href`/`template`/anchor
/// decisions inside table markup.
struct LinkTarget {
    /// True when the link has either an `href` or `template` value.
    has_value: bool,

    /// Display label for the target kind.
    label: &'static str,

    /// Target value shown and copied by the UI.
    value: String,

    /// True when `href` is a concrete HTTP(S) URL.
    has_href: bool,

    /// Anchor destination when `has_href` is true.
    href: String,
}

impl LinkTarget {
    /// Extracts the primary target from a JRD link object.
    ///
    /// `href` wins over `template` because it is the concrete target to inspect first. Templates
    /// remain visible but intentionally do not become clickable anchors.
    fn from_object(object: &serde_json::Map<String, Value>) -> Self {
        if let Some(href) = object.get("href").and_then(Value::as_str) {
            let has_href = clickable_href(href);
            return Self {
                has_value: true,
                label: "href",
                value: href.to_string(),
                has_href,
                href: href.to_string(),
            };
        }

        if let Some(template) = object.get("template").and_then(Value::as_str) {
            return Self {
                has_value: true,
                label: "template",
                value: template.to_string(),
                has_href: false,
                href: String::new(),
            };
        }

        Self {
            has_value: false,
            label: "",
            value: String::new(),
            has_href: false,
            href: String::new(),
        }
    }
}

/// Collects non-standard link fields for the optional `extra` table column.
///
/// Common fields get first-class columns, while uncommon fields stay visible without changing the
/// table shape for every response. Validate this by checking a response with custom link metadata
/// and confirming the standard rel/type/href/template fields are not duplicated in `extra`.
fn extra_fields(object: &serde_json::Map<String, Value>) -> String {
    object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "rel" | "type" | "href" | "template"))
        .map(|(key, value)| format!("{key}: {}", string_value(value)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns pretty JSON when available, otherwise the captured raw body.
///
/// Pretty JSON is the best secondary view for valid JRD responses. Falling back to raw text keeps
/// HTML errors, plain-text failures, and truncated non-JSON responses copyable for debugging.
pub fn raw_body(result: &LookupResult) -> String {
    if let Some(json) = &result.json {
        serde_json::to_string_pretty(json).unwrap_or_else(|_| result.body.clone())
    } else {
        result.body.clone()
    }
}

/// Renders a JSON scalar or object as a compact display string.
///
/// String values should not be re-quoted in the summary tables. Non-string values keep their JSON
/// representation so booleans, numbers, arrays, and objects are still distinguishable at a glance.
fn string_value(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}

/// Returns true when a value should be rendered as a clickable link.
///
/// WebFinger templates contain URI variables and should remain plain text. Only concrete HTTP(S)
/// `href` values become anchors.
fn clickable_href(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_empty_state_for_non_json() {
        let summary = SummaryView::from_json(None);

        assert!(summary.has_empty_message);
        assert_eq!(
            summary.empty_message,
            "No JSON Resource Descriptor was parsed."
        );
    }

    #[test]
    fn link_href_is_clickable_but_template_is_not() {
        let json: Value = serde_json::json!({
            "links": [
                {"rel": "profile", "href": "https://example.com/@alice", "type": "text/html"},
                {"rel": "subscribe", "template": "https://example.com/authorize?uri={uri}"}
            ]
        });
        let summary = SummaryView::from_json(Some(&json));

        assert!(summary.links[0].has_target_href);
        assert!(!summary.links[1].has_target_href);
    }

    #[test]
    fn extra_fields_skip_standard_link_columns() {
        let json: Value = serde_json::json!({
            "links": [
                {
                    "rel": "self",
                    "href": "https://example.com/users/alice",
                    "type": "application/activity+json",
                    "titles": {"en": "Alice"}
                }
            ]
        });
        let summary = SummaryView::from_json(Some(&json));

        assert!(summary.has_extra_fields);
        assert_eq!(summary.links[0].extra, "titles: {\"en\":\"Alice\"}");
    }
}
