use nutype::nutype;

/// Link relation type
///
/// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.1>
#[nutype(derive(
    Debug,
    Display,
    Clone,
    From,
    Into,
    FromStr,
    Display,
    Serialize,
    Deserialize,
    AsRef,
    Deref,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
))]
pub struct Rel(String);
