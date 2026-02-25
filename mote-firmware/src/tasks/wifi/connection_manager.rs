use alloc::string::String;
use core::cmp::min;

use cyw43::JoinOptions;
use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use mote_api::messages::host_to_mote::SetNetworkConnectionConfig;
use mote_api::messages::mote_to_host::{BITResult, NetworkConnection};
use {defmt_rtt as _, panic_probe as _};

use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;

pub static WIFI_REQUEST_CONNECT: Channel<CriticalSectionRawMutex, SetNetworkConnectionConfig, 1> = Channel::new();
pub static WIFI_REQUEST_RESCAN: Signal<CriticalSectionRawMutex, ()> = Signal::new();

async fn attempt_join_network<'a>(control: &mut cyw43::Control<'a>, config: SetNetworkConnectionConfig) {
    async fn update_network_bit(current_network: Option<String>, result: BITResult) {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.current_network_connection = current_network;
        update_bit_result(
            &mut configuration_state.built_in_test.wifi,
            "Connected to Network",
            result,
        );
    }

    update_network_bit(None, BITResult::Waiting).await;

    for attempt in 1..=3 {
        if config.password.len() >= 8 {
            info!("Attempting network join using WPA2+WPA3 with AES");
            if let Err(err) = control
                .join(&config.ssid, JoinOptions::new(config.password.as_bytes()))
                .await
            {
                info!("join failed with status={}, attempt {} / 3", err.status, attempt);
                continue;
            }

            update_network_bit(Some(config.ssid), BITResult::Pass).await;
            return;
        } else {
            info!("Attempting network join as open");
            if let Err(err) = control.join(&config.ssid, JoinOptions::new_open()).await {
                info!("join failed with status={}, attempt {} / 3", err.status, attempt);
                continue;
            }

            update_network_bit(Some(config.ssid), BITResult::Pass).await;
            return;
        }
    }

    update_network_bit(Some(config.ssid), BITResult::Fail).await;
}

async fn run_network_scan<'a>(control: &mut cyw43::Control<'a>) {
    // Clear previous scan
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.available_network_connections.clear();
    }

    let mut scanner = control.scan(Default::default()).await;
    while let Some(bss) = scanner.next().await {
        if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
            if bss.ssid.iter().all(|&n| n == 0) {
                continue;
            }

            // Update available networks
            {
                let mut configuration_state = CONFIGURATION_STATE.lock().await;

                let new_connection = NetworkConnection {
                    ssid: ssid_str.into(),
                    strength: -bss.rssi as u8,
                };

                // Check if this network is already listed
                if let Some(item) = configuration_state
                    .available_network_connections
                    .iter_mut()
                    .find(|i| i.ssid == new_connection.ssid)
                {
                    item.strength = min(item.strength, new_connection.strength);
                    continue;
                }

                // If we've run out of entries, drop the weakest
                if configuration_state.available_network_connections.len() > 50 {
                    let (weakest_index, weakest) = configuration_state
                        .available_network_connections
                        .iter()
                        .enumerate()
                        .max_by_key(|&(_index, val)| val.strength)
                        .unwrap();

                    if weakest.strength > new_connection.strength {
                        configuration_state.available_network_connections.remove(weakest_index);
                    }
                }

                configuration_state.available_network_connections.push(new_connection);
            }
        }
    }
}

#[embassy_executor::task]
pub async fn connection_manager_task(mut control: cyw43::Control<'static>) -> ! {
    // Populate network scan state
    run_network_scan(&mut control).await;

    // Attempt to join whatever network is saved in flash
    // TODO: Load config from flash, then attempt connect
    // attempt_join_network(&mut control).await;

    loop {
        match select(WIFI_REQUEST_CONNECT.receive(), WIFI_REQUEST_RESCAN.wait()).await {
            Either::First(config) => {
                info!(
                    "Got join request {}, {}",
                    config.ssid.as_str(),
                    config.password.as_str()
                );
                attempt_join_network(&mut control, config).await;
            }
            Either::Second(_) => run_network_scan(&mut control).await,
        }
    }
}
