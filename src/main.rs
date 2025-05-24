use anyhow::Result;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use tokio::net::TcpListener;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;

use tokio::signal;

use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server_addr = "127.0.0.1:8443";

    let listener = TcpListener::bind(server_addr).await?;
    let tls_acceptor = load_tls_config()?;
    tracing::info!("Listening with TLS on {}", server_addr);

    loop {
        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
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

            _ = signal::ctrl_c() => {
                tracing::info!("Shutting down");
                break;
            }
        }
    }

    tracing::info!("Shut down successfully");

    Ok(())
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
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    while let Some(line) = framed.next().await {
        let bytes = line?;
        let line = String::from_utf8_lossy(&bytes);
        let span = tracing::info_span!("handle_message", message = %line);
        let _enter = span.enter();

        tracing::debug!("Processing message");

        let resp = handle_msg(&bytes).await?;
        let resp_bytes = bincode::serialize(&resp)?;

        framed.send(resp_bytes.into()).await?;
    }

    tracing::info!("Client disconnected");

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse(String);

#[typetag::serde]
impl Response for ErrorResponse {}

async fn handle_msg(input: &[u8]) -> Result<Box<dyn Response>> {
    let req: Box<dyn Request> = match bincode::deserialize(input) {
        Ok(r) => r,
        Err(e) => {
            return Ok(Box::new(ErrorResponse(format!(
                "Failed to parse request: {e}"
            ))));
        }
    };
    req.handle().await
}

use serde::{Deserialize, Serialize};

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait Request: Send + Sync {
    async fn handle(&self) -> Result<Box<dyn Response>>;
}

#[typetag::serde(tag = "type")]
pub trait Response: Send + Sync {}

#[derive(Serialize, Deserialize)]
pub struct Ping;

#[derive(Serialize, Deserialize)]
pub struct PingResponse(String);

#[typetag::serde]
impl Response for PingResponse {}

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Ping {
    async fn handle(&self) -> Result<Box<dyn Response>> {
        Ok(Box::new(PingResponse(
            "Thou shalt not to use HTTP;\nThou shalt write thoust own protocol".to_string(),
        )))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Echo {
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct EchoResponse(String);

#[typetag::serde]
impl Response for EchoResponse {}

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Echo {
    async fn handle(&self) -> Result<Box<dyn Response>> {
        Ok(Box::new(EchoResponse(self.message.clone())))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Add {
    pub a: i32,
    pub b: i32,
}

#[derive(Serialize, Deserialize)]
pub struct AddResponse {
    sum: i32,
}

#[typetag::serde]
impl Response for AddResponse {}

#[typetag::serde]
#[async_trait::async_trait]
impl Request for Add {
    async fn handle(&self) -> Result<Box<dyn Response>> {
        Ok(Box::new(AddResponse {
            sum: self.a + self.b,
        }))
    }
}
