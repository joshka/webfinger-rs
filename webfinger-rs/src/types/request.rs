use http::Uri;

use crate::Rel;

#[derive(Debug)]
pub struct Request {
    /// Query target.
    ///
    /// This is the URI of the resource to query. It will be stored in the `resource` query
    /// parameter.
    ///
    /// TODO: This could be a newtype that represents the resource and makes it easier to extract
    /// the values / parse into the right types (e.g. `acct:` URIs).
    pub resource: Uri,

    /// The host to query
    ///
    /// TODO: this might be better as an Option<Uri> or Option<Host> or something similar. When the
    /// resource has a host part, it should be used unless this field is set.
    pub host: String,

    /// Link relation types
    ///
    /// This is a list of link relation types to query for. Each link relation type will be stored
    /// in a `rel` query parameter.
    pub rels: Vec<Rel>,
}

impl Request {
    pub fn new(resource: Uri) -> Self {
        Self {
            host: String::new(),
            resource,
            rels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::*;

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.1
    #[test]
    fn example_3_1() {
        let resource = "acct:carol@example.com".parse().unwrap();
        let rel = Rel::from("http://openid.net/specs/connect/1.0/issuer");
        let host = "example.com".parse().unwrap();
        let query = Request {
            host,
            resource,
            rels: vec![rel],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // `"/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fopenid.net%2Fspecs%2Fconnect%2F1.0%2Fissuer"`
        assert_eq!(
            uri.to_string(),
            "https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://openid.net/specs/connect/1.0/issuer",
            );
    }

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.2
    #[test]
    fn example_3_2() {
        let resource = "http://blog.example.com/article/id/314".parse().unwrap();
        let query = Request {
            host: "blog.example.com".parse().unwrap(),
            resource,
            rels: vec![],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // /.well-known/webfinger?resource=http%3A%2F%2Fblog.example.com%2Farticle%2Fid%2F314
        assert_eq!(
            uri.to_string(),
            "https://blog.example.com/.well-known/webfinger?resource=http://blog.example.com/article/id/314",
        );
    }
}
