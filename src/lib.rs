use anyhow::Result;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};

use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub async fn handle_client<S>(stream: S, peer_addr: std::net::SocketAddr) -> Result<()>
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
