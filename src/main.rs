use anyhow::Result;
use tokio::net::TcpListener;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};

use tokio::signal;

use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server_addr = "127.0.0.1:8443";

    let listener = TcpListener::bind(server_addr).await?;
    tracing::info!("Listening on {}", server_addr);

    loop {
        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
                tracing::info!(%addr, "Client connected");

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr).await {
                        tracing::error!(%addr, error = %e, "Error handling client");
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

async fn handle_client<S>(stream: S, peer_addr: std::net::SocketAddr) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let span = tracing::info_span!("client_session", %peer_addr);
    let _enter = span.enter();

    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    while let Some(line) = framed.next().await {
        let bytes = line?;
        let line = String::from_utf8_lossy(&bytes);

        let msg_span = tracing::info_span!("handle_message", message = %line);
        let _enter_msg = msg_span.enter();

        tracing::debug!("Processing message");

        let resp = handle_msg(&bytes).await?;
        let resp_bytes = bincode::serialize(&resp)?;

        framed.send(resp_bytes.into()).await?;
    }

    tracing::info!("Client disconnected");

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
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

#[typetag::serde]
#[async_trait::async_trait]
pub trait Request: Send + Sync + std::fmt::Debug {
    async fn handle(&self) -> Result<Box<dyn Response>>;
}

#[typetag::serde]
pub trait Response: Send + Sync + std::fmt::Debug {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ping;

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Echo {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Add {
    pub a: i32,
    pub b: i32,
}

#[derive(Serialize, Deserialize, Debug)]
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
