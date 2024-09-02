use std::{borrow::BorrowMut, cell::RefCell, marker::PhantomData, rc::Rc};

use codee::binary::BincodeSerdeCodec;
use common::{endpoints, message::Message, params::HostParams};
use leptos::{
    create_effect, logging::warn, on_cleanup, store_value, Signal, SignalGet, StoredValue,
};
use leptos_use::{use_websocket, UseWebSocketReturn};
use thiserror::Error;
use tracing::info;

#[derive(Clone)]
pub struct RoomManager {
    state: StoredValue<RoomState<Message>>,
}

pub enum RoomState<Tx>
where
    Tx: 'static,
{
    Disconnected,
    Connected(WebsocketContext<Tx>),
}

#[derive(Error, Debug)]
pub enum RoomManagerError {
    #[error("already connected to room")]
    AlreadyConnectedToRoom,
    #[error("not connected to room")]
    NotConnectedToRoom,

    #[error("Param failed to encode")]
    ParamError(#[from] serde_urlencoded::ser::Error),
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            state: store_value(RoomState::Disconnected),
        }
    }

    pub fn host(&self, name: String) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let is_connected = self
            .state
            .with_value(|v| matches!(v, RoomState::Connected(_)));
        if is_connected {
            return Err(RoomManagerError::AlreadyConnectedToRoom);
        }
        let host_params = HostParams { name };
        let params = serde_urlencoded::to_string(&host_params);
        match params {
            Ok(params) => {
                let UseWebSocketReturn { send, message, .. } =
                    use_websocket::<Message, Message, BincodeSerdeCodec>(&format!(
                        "{}?{params}",
                        endpoints::HOST_ROOM
                    ));
                let messages_c = message.clone();
                create_effect(move |_| {
                    info!("Received message {:#?}", messages_c.get());
                });
                // info!("is connecting {is_connecting}");
                self.state.update_value(|val| {
                    *val = RoomState::Connected(WebsocketContext::new(
                        message.clone(),
                        Box::new(send),
                    ));
                });
                Ok(message)
            }
            Err(err) => {
                warn!("Cant serialize params {err:#?}");
                Err(err.into())
            }
        }
    }

    pub fn message_signal(&self) -> Result<Signal<Option<Message>>, RoomManagerError> {
        self.state.with_value(|val| match val {
            RoomState::Disconnected => Err(RoomManagerError::NotConnectedToRoom),
            RoomState::Connected(connection) => Ok(connection.message.clone()),
        })
    }
}

pub struct WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub message: Signal<Option<Message>>,
    send: Box<dyn Fn(&Tx)>, // use Rc to make it easily cloneable
    _phantom: PhantomData<Tx>,
}

impl<Tx> WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub fn new(message: Signal<Option<Message>>, send: Box<dyn Fn(&Tx)>) -> Self {
        Self {
            message,
            send,
            _phantom: PhantomData,
        }
    }

    // create a method to avoid having to use parantheses around the field
    #[inline(always)]
    pub fn send(&self, message: &Tx) {
        (self.send)(&message)
    }
}
