use anyhow::Result;
use tokio::net::TcpListener;

use tokio::signal;

use myproto::*;
use serde::{Deserialize, Serialize};

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
