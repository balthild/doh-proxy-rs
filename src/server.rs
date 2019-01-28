use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use futures::prelude::*;
use futures_01::{Future as Future01, Stream as Stream01};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;
use lazy_static::lazy_static;
use native_tls::{Identity, TlsAcceptor};
use tokio::await;
use tokio::fs::File;
use tokio::net::{TcpListener, UdpSocket};

const MIN_DNS_QUESTION_LEN: usize = 17;
const MAX_DNS_QUESTION_LEN: usize = 4096;
const MAX_DNS_RESPONSE_LEN: usize = 4096;

lazy_static! {
    static ref UPSTREAM: SocketAddr = {
        crate::args::ARGS.upstream
            .parse().expect("Invalid upstream address")
    };
    static ref LOCAL: SocketAddr = {
        if UPSTREAM.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }
            .parse().unwrap()
    };
}

pub fn run(listen_addr: &str) {
    let args = &*crate::args::ARGS;

    if args.no_tls {
        println!("WARNING: HTTPS disabled");
    } else if args.identity.is_empty() {
        panic!("You must specify TLS identity or disable HTTPS");
    };

    println!("Running DoH server. Upstream DNS: {}", args.upstream);

    let listen = listen_addr.parse().expect("Invalid listen address");

    if args.no_tls {
        let future = run_http_server(listen);
        hyper::rt::run(future.unit_error().boxed().compat());
    } else {
        let future = run_https_server(listen);
        hyper::rt::run(future.unit_error().boxed().compat());
    }
}

async fn run_http_server(listen: SocketAddr) {
    println!("Listening on http://{}", listen);

    let serve_future = Server::bind(&listen)
        .serve(|| service_fn(|req|
            serve_req(req).boxed().compat()
        ));

    if let Err(e) = await!(serve_future) {
        eprintln!("Server error: {}", e);
    }
}

async fn run_https_server(listen: SocketAddr) {
    println!("Listening on https://{}", listen);

    let identity = await!(load_tls_identity());
    let tls = TlsAcceptor::builder(identity).build().unwrap();
    let tls = tokio_tls::TlsAcceptor::from(tls);

    let listener = TcpListener::bind(&listen)
        .expect(&format!("Failed to listen on {}", listen));

    let incoming = listener.incoming()
        .and_then(move |socket| {
            tls.accept(socket).map_err(|e| Error::new(ErrorKind::Other, e))
        });

    let serve_future = Server::builder(incoming)
        .serve(|| service_fn(|req|
            serve_req(req).boxed().compat()
        ));

    if let Err(e) = await!(serve_future) {
        eprintln!("Server error: {}", e);
    }
}

async fn load_tls_identity() -> Identity {
    let args = &*crate::args::ARGS;

    let file = await!(File::open(&args.identity))
        .expect(&format!("Cannot open PKCS#12 file: {}", args.identity));

    let (_, pkcs12) = await!(tokio::io::read_to_end(file, vec![]))
        .expect(&format!("Cannot read PKCS#12 file: {}", args.identity));

    Identity::from_pkcs12(&pkcs12, &args.password)
        .expect(&format!("Cannot load PKCS#12 file: {}", args.identity))
}

async fn serve_req<'a>(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    if req.uri().path() != "/dns-query" {
        return Ok(abort(StatusCode::NOT_FOUND));
    }

    let method = req.method();
    let question = match method {
        &Method::GET => req.uri().query().and_then(get_question),
        &Method::POST => Some(await!(req.into_body().concat2())?.to_vec()),
        _ => return Ok(abort(StatusCode::METHOD_NOT_ALLOWED)),
    };

    let answer = match question {
        Some(data) => {
            if data.len() > MAX_DNS_QUESTION_LEN {
                return Ok(abort(StatusCode::PAYLOAD_TOO_LARGE));
            } else if data.len() < MIN_DNS_QUESTION_LEN {
                return Ok(abort(StatusCode::BAD_REQUEST));
            }

            await!(ask_upstream(data))
        }
        None => return Ok(abort(StatusCode::BAD_REQUEST)),
    };

    match answer {
        Some(data) => {
            let ttl = match dns_parser::Packet::parse(&data) {
                Ok(p) => p.answers.iter().map(|r| r.ttl).min().unwrap_or(1),
                Err(_) => return Ok(abort(StatusCode::BAD_GATEWAY)),
            };

            Ok(Response::builder()
                .header("Cache-Control", format!("max-age={}", ttl))
                .body(Body::from(data))
                .unwrap())
        }
        None => Ok(abort(StatusCode::BAD_GATEWAY)),
    }
}

fn abort(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap()
}

fn get_question(query_str: &str) -> Option<Vec<u8>> {
    for param in query_str.split('&') {
        let mut pair = param.split('=');

        if Some("dns") == pair.next() {
            let val = pair.next()?;
            let val = val.replace("\r", "");
            return base64::decode(&val).ok();
        }
    }

    None
}

async fn ask_upstream(question: Vec<u8>) -> Option<Vec<u8>> {
    let socket = UdpSocket::bind(&LOCAL).ok()?;

    let (socket, _) = await!(socket.send_dgram(question, &UPSTREAM)).ok()?;

    let data = vec![0u8; MAX_DNS_RESPONSE_LEN];
    let (_, data, _, _) = await!(socket.recv_dgram(data)).ok()?;

    Some(data)
}
