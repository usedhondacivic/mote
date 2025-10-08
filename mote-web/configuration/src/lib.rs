use wasm_bindgen::prelude::*;

use mote_messages::configuration::{host_to_mote, mote_to_host};
use mote_sansio_driver::{HostConfigurationLink, SerialEndpoint};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct ConfigurationLink {
    link: HostConfigurationLink,
}

// WASM Wrapper for HostConfigurationLink
// TODO: Handle errors here
#[wasm_bindgen]
impl ConfigurationLink {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            link: HostConfigurationLink::new(),
        }
    }

    pub fn send(&mut self, msg: JsValue) {
        console_log!("[TX] Configuration link send: {:?}", msg);
        let message: host_to_mote::Message = serde_wasm_bindgen::from_value(msg).unwrap();
        console_log!("[TX] Configuration link unpacked: {:?}", message);
        self.link.send(SerialEndpoint, message).unwrap();
        console_log!("[TX] Message queued for send");
    }

    pub fn poll_transmit(&mut self) -> JsValue {
        if let Some(transmit) = self.link.poll_transmit() {
            console_log!("[TX] Sending {:?}", transmit.payload);
            serde_wasm_bindgen::to_value(&transmit.payload).unwrap()
        } else {
            serde_wasm_bindgen::to_value(&()).unwrap()
        }
    }

    pub fn handle_receive(&mut self, bytes: JsValue) -> JsValue {
        let mut bytes: Vec<u8> = serde_wasm_bindgen::from_value(bytes).unwrap();
        console_log!("[RX] Configuration link received: {:?}", bytes);
        self.link.handle_receive(&mut bytes);
        let message: Result<Option<mote_to_host::Message>, _> = self.link.poll_receive();
        console_log!("[RX] Configuration link unpacked: {:?}", message);
        serde_wasm_bindgen::to_value(&message).unwrap()
    }
}
