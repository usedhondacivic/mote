use defmt::error;
use mote_api::messages::mote_to_host::{BITList, BITResult};

pub fn update_bit_result(collection: &mut BITList, name: &'static str, result: BITResult) {
    if let Some(bit) = collection.iter_mut().find(|i| i.name == name) {
        bit.result = result;
    } else {
        error!("Failed to update BIT result for {}", name);
    }
}
