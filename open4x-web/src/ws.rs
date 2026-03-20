//! WebSocket client for communicating with the open4x-server.

use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use open4x_api::messages::{ClientMessage, ServerMessage};

/// Shared WebSocket handle for sending messages from UI event handlers.
#[derive(Clone)]
pub struct WsClient {
    socket: Rc<WebSocket>,
}

impl WsClient {
    /// Connect to the server and return a client handle.
    ///
    /// `on_msg` is called for each deserialized `ServerMessage` received.
    /// Returns `None` if the WebSocket connection cannot be established.
    pub fn connect(url: &str, on_msg: impl Fn(ServerMessage) + 'static) -> Option<Self> {
        let ws = WebSocket::new(url).ok()?;

        // Set up onmessage callback.
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
            if let Some(text) = evt.data().as_string() {
                match serde_json::from_str::<ServerMessage>(&text) {
                    Ok(msg) => on_msg(msg),
                    Err(e) => web_sys::console::warn_1(
                        &format!("ws: failed to parse message: {e}").into(),
                    ),
                }
            }
        });
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget(); // leak closure so it lives as long as the socket

        Some(Self { socket: Rc::new(ws) })
    }

    /// Send a `ClientMessage` as JSON over the WebSocket.
    pub fn send(&self, msg: &ClientMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.socket.send_with_str(&json);
        }
    }

    /// Returns true if the socket is in the OPEN state.
    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.socket.ready_state() == WebSocket::OPEN
    }
}
