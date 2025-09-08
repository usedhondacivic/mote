use postcard::from_bytes;
use wasm_bindgen::prelude::*;

use mote_messages::configuration::{
    host_to_mote,
    mote_to_host::{self, NetworkConnection},
};
use mote_sansio_driver::{HostConfigurationLink, SerialEndpoint};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
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
        console_log!("Send: {:?}", msg);
        let message: host_to_mote::Message = serde_wasm_bindgen::from_value(msg).unwrap();
        console_log!("Unpacked: {:?}", message);
        self.link.send(SerialEndpoint, message).unwrap();
    }

    pub fn poll_transmit(&mut self) -> JsValue {
        let transmit = self.link.poll_transmit().unwrap();

        serde_wasm_bindgen::to_value(&transmit.payload).unwrap()
    }

    pub fn handle_receive(&self, bytes: JsValue) -> JsValue {
        console_log!("Recv: {:?}", bytes);
        let bytes: heapless::Vec<u8, 1500> = serde_wasm_bindgen::from_value(bytes.clone()).unwrap();
        let message: mote_to_host::Message = from_bytes(&bytes).unwrap();
        console_log!("Unpacked: {:?}", message);

        serde_wasm_bindgen::to_value(&message).unwrap()
    }
}
