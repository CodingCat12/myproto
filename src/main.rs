use anyhow::{Error, Result};
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use serde::ser::SerializeMap;
use tokio::net::TcpListener;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let listener = TcpListener::bind("127.0.0.1:8443").await?;
    let tls_acceptor = load_tls_config()?;
    tracing::info!("Listening with TLS on 127.0.0.1:8443");

    loop {
        let (stream, addr) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();

        tracing::info!(%addr, "Client connected");

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    if let Err(e) = handle_client(tls_stream).await {
                        tracing::error!(%addr, error = %e, "Error handling client");
                    }
                }
                Err(e) => {
                    tracing::error!(%addr, error = %e, "TLS handshake failed");
                }
            }
        });
    }
}

use std::sync::Arc;

fn load_tls_config() -> anyhow::Result<TlsAcceptor> {
    let certs: Vec<_> =
        CertificateDer::pem_file_iter("cert.pem")?.collect::<Result<Vec<_>, _>>()?;

    let private_key = PrivateKeyDer::from_pem_file("key.pem")?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

async fn handle_client<S>(stream: S) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut framed = Framed::new(stream, LinesCodec::new());

    while let Some(line) = framed.next().await {
        let line = line?;
        let span = tracing::info_span!("handle_message", message = %line);
        let _enter = span.enter();

        tracing::debug!("Processing message");

        let resp = match handle_msg(line.as_bytes()).await {
            Ok(s) => Response::Success(s),
            Err(err) => Response::Error(err),
        };

        let resp_str = serde_json::to_string_pretty(&resp)?;
        framed.send(resp_str).await?;
    }

    Ok(())
}

pub enum Response {
    Success(Box<dyn erased_serde::Serialize + Send + Sync>),
    Error(Error),
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        match self {
            Response::Success(data) => {
                map.serialize_entry("type", "Success")?;
                map.serialize_entry("value", &Erased(data))?;
            }
            Response::Error(e) => {
                map.serialize_entry("type", "Error")?;
                map.serialize_entry("message", &e.to_string())?;
            }
        }

        map.end()
    }
}
struct Erased<'a>(&'a dyn erased_serde::Serialize);

impl<'a> Serialize for Erased<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        erased_serde::serialize(self.0, serializer)
    }
}

async fn handle_msg(input: &[u8]) -> Result<Box<dyn erased_serde::Serialize + Send + Sync>> {
    let req: Box<dyn Request> = serde_json::from_slice(input)?;
    req.handle().await
}

use serde::{Deserialize, Serialize, Serializer};

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait Request: Send + Sync {
    async fn handle(&self) -> Result<Box<dyn erased_serde::Serialize + Send + Sync>>;
}

#[derive(Serialize, Deserialize)]
pub struct Ping;

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Ping {
    async fn handle(&self) -> Result<Box<dyn erased_serde::Serialize + Send + Sync>> {
        Ok(Box::new(
            "Thou shalt not to use HTTP;\nThou shalt write thoust own protocol".to_string(),
        ))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Echo {
    pub message: String,
}

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Echo {
    async fn handle(&self) -> Result<Box<dyn erased_serde::Serialize + Send + Sync>> {
        Ok(Box::new(self.message.clone()))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Add {
    pub a: i32,
    pub b: i32,
}

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Add {
    async fn handle(&self) -> Result<Box<dyn erased_serde::Serialize + Send + Sync>> {
        Ok(Box::new(self.a + self.b))
    }
}
