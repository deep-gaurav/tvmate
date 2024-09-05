use tracing::warn;

use crate::message::Message;

pub trait MessageSender {
    #[allow(async_fn_in_trait)]
    async fn send_binary(&mut self, data: Vec<u8>);

    #[allow(async_fn_in_trait)]
    async fn send_message(&mut self, message: &Message) {
        let data = bincode::serialize(message);
        match data {
            Ok(data) => {
                self.send_binary(data).await;
            }
            Err(err) => {
                warn!("Failed to serialize message {err:#?}");
            }
        }
    }
}

#[cfg(feature = "ssr")]
impl MessageSender for axum::extract::ws::WebSocket {
    async fn send_binary(&mut self, data: Vec<u8>) {
        if let Err(err) = self.send(axum::extract::ws::Message::Binary(data)).await {
            warn!("Failed to send message {err:#?}")
        }
    }
}
