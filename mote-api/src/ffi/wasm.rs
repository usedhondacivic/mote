//! Foreign function interfaces for TypeScript (WASM)

use wasm_bindgen::prelude::*;

use gloo_utils::format::JsValueSerdeExt;

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::MoteConfigLink;
use crate::messages::{host_to_mote, mote_to_host};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[allow(dead_code)]
#[wasm_bindgen]
struct Link {
    link: MoteConfigLink,
}
#[allow(dead_code)]
#[wasm_bindgen]
impl Link {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            link: MoteConfigLink::new(),
        }
    }

    pub fn send(&mut self, msg: JsValue) {
        console_log!("[TX] Configuration link send: {:?}", msg);
        let message: host_to_mote::Message = JsValue::into_serde(&msg).unwrap();
        console_log!("[TX] Configuration link unpacked: {:?}", message);
        self.link.send(message).unwrap();
        console_log!("[TX] Message queued for send");
    }

    pub fn poll_transmit(&mut self) -> JsValue {
        if let Some(transmit) = self.link.poll_transmit() {
            console_log!("[TX] Sending {:?}", transmit.payload);
            JsValue::from_serde(&transmit.payload).unwrap()
        } else {
            JsValue::from_serde(&()).unwrap()
        }
    }

    pub fn handle_receive(&mut self, bytes: JsValue) {
        let bytes: Vec<u8> = JsValue::into_serde(&bytes).unwrap();
        console_log!("[RX] Configuration link received: {:?}", bytes);
        self.link.handle_receive(&bytes);
    }

    pub fn poll_receive(&mut self) -> JsValue {
        let message: Result<Option<mote_to_host::Message>, _> = self.link.poll_receive();
        console_log!("[RX] Configuration link unpacked: {:?}", message);
        JsValue::from_serde(&message.map_err(|_| ())).unwrap()
    }
}
