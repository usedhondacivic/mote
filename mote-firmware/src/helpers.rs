use mote_messages::configuration::mote_to_host::{BITList, BITResult};

pub fn update_bit_result(collection: &mut BITList, name: &'static str, result: BITResult) {
    collection.iter_mut().find(|i| i.name == name).unwrap().result = result;
}
