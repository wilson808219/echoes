use crate::support::factories::header;
use crate::support::io::IO;
use crate::support::resp::{empty, error, forbidden};
use crate::support::tls::connector;
use bytes::Bytes;
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Error, Method, Request, Response};
use log::{debug, error, info};
use rustls::pki_types::ServerName;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

mod support;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    support::logger::init().await?;
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await?;
    info!("forwarding service listening on (https://{})", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        spawn(async move {
            let io = IO::new(stream);
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(proxy))
                .with_upgrades()
                .await
            {
                error!("failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn proxy(
    mut request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    if Method::CONNECT == request.method() {
        if let Some(addr) = host(request.uri()) {
            spawn(async move {
                match hyper::upgrade::on(request).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr).await {
                            error!("client io error: {}", e);
                        }
                    }
                    Err(e) => error!("upgrade error: {}", e),
                }
            });
            Ok(Response::new(empty()))
        } else {
            error!("CONNECT host is not socket addr: {:?}", request.uri());
            Ok(error(format!(
                "CONNECT must be to a socket address: {}",
                request.uri()
            )))
        }
    } else {
        let headers = request.headers();
        if let Some(sc) = headers.get("x-sc") {
            if let Ok(sc) = sc.to_str() {
                let host = format!("{}.hsse.sudti.cn", sc);
                info!("try forwarding to: {}", host);
                let connector = connector().await;
                match TcpStream::connect((host.clone(), 443)).await {
                    Ok(stream) => match ServerName::try_from(host.clone()) {
                        Ok(servername) => match connector.connect(servername, stream).await {
                            Ok(stream) => {
                                let io = IO::new(stream);
                                let (mut sender, conn) = Builder::new()
                                    .preserve_header_case(true)
                                    .title_case_headers(true)
                                    .handshake(io)
                                    .await?;
                                spawn(async move {
                                    if let Err(err) = conn.await {
                                        error!("access failed: {:?}", err);
                                    }
                                });
                                header(request.headers_mut(), &host);
                                let resp = sender.send_request(request).await?;
                                info!("forwarded to: {}, status: {}", host, resp.status());
                                Ok(resp.map(|b| b.boxed()))
                            }
                            Err(err) => Ok(error(format!("TLS handshake failed: {}", err))),
                        },
                        Err(err) => Ok(error(format!("unknown host: {}", err))),
                    },
                    Err(err) => Ok(error(format!("access failed: {}", err))),
                }
            } else {
                Ok(forbidden())
            }
        } else {
            Ok(forbidden())
        }
    }
}

fn host(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = IO::new(upgraded);
    let (client, server) = tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
    debug!(
        "client wrote {} bytes and received {} bytes",
        client, server
    );
    Ok(())
}
