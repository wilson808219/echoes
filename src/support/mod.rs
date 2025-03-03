pub(crate) mod executor;
pub(crate) mod factories;
pub(crate) mod io;
pub(crate) mod logger;

pub(crate) mod tls {
    use rustls::ClientConfig;
    use std::sync::{Arc, OnceLock};
    use tokio_rustls::TlsConnector;

    pub(crate) static CONNECTOR: OnceLock<TlsConnector> = OnceLock::new();

    #[inline]
    pub(crate) async fn connector() -> &'static TlsConnector {
        let connector = CONNECTOR.get_or_init(|| {
            TlsConnector::from(Arc::new(
                ClientConfig::builder()
                    .with_root_certificates(cert())
                    .with_no_client_auth(),
            ))
        });
        connector
    }

    fn cert() -> rustls::RootCertStore {
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(
            rustls_native_certs::load_native_certs()
                .expect("could not load platform certs")
                .into_iter()
                .map(|cert| cert.into())
                .collect::<Vec<_>>(),
        );
        roots
    }
}

pub(crate) mod resp {
    use bytes::Bytes;
    use http::{HeaderValue, Response};
    use http_body_util::combinators::BoxBody;
    use http_body_util::{BodyExt, Empty, Full};
    use hyper::Error;
    use log::error;
    use serde::{Deserialize, Serialize};

    #[inline]
    pub(crate) fn forbidden() -> Response<BoxBody<Bytes, Error>> {
        error!("missing header(x-sc)");
        let mut resp = Response::new(full(
            R::error("missing header(x-sc)", "缺少服务编码").serialize(),
        ));
        resp.headers_mut().insert(
            "content-type",
            HeaderValue::from_str("application/json").unwrap(),
        );
        *resp.status_mut() = http::StatusCode::BAD_REQUEST;
        resp
    }

    #[inline]
    pub(crate) fn error(err: &str) -> Response<BoxBody<Bytes, Error>> {
        error!("error on: {}", err);
        let mut resp = Response::new(full(R::err(err).serialize()));
        resp.headers_mut().insert(
            "content-type",
            HeaderValue::from_str("application/json").unwrap(),
        );
        *resp.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
        resp
    }

    #[inline]
    pub(crate) fn empty() -> BoxBody<Bytes, Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }

    #[inline]
    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
        Full::new(chunk.into())
            .map_err(|never| match never {})
            .boxed()
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct R {
        pub(crate) success: bool,
        pub(crate) error: Option<String>,
        pub(crate) error_description: Option<String>,
    }

    impl R {
        #[inline]
        pub(crate) fn new(
            success: bool,
            error: Option<String>,
            error_description: Option<String>,
        ) -> Self {
            R {
                success,
                error,
                error_description,
            }
        }

        #[inline]
        pub fn ok() -> Self {
            R {
                success: true,
                error: None,
                error_description: None,
            }
        }

        #[inline]
        pub fn err(err: &str) -> Self {
            R {
                success: false,
                error: Some(String::from(err)),
                error_description: Some(String::from(err)),
            }
        }

        #[inline]
        pub fn error(err: &str, desc: &str) -> Self {
            R {
                success: false,
                error: Some(String::from(err)),
                error_description: Some(String::from(desc)),
            }
        }

        pub fn serialize(self) -> String {
            serde_json::to_string(&self).map_or_else(
                |_| {
                    error!("failed to serialize frame");
                    String::from("")
                },
                |v| v,
            )
        }
    }
}
