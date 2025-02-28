use http::{HeaderMap, HeaderValue};

#[inline]
pub fn header(headers: &mut HeaderMap<HeaderValue>, host: &str) {
    headers.remove("x-sc");
    if let Ok(value) = HeaderValue::from_str(host) {
        headers.insert("host", value);
    }
}
