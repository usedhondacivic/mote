use alloc::string::String;
use core::cmp::min;

use cyw43::JoinOptions;
use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, with_timeout};
use mote_api::messages::host_to_mote::SetNetworkConnectionConfig;
use mote_api::messages::mote_to_host::{BITResult, NetworkConnection};
use {defmt_rtt as _, panic_probe as _};

use crate::flash_config;
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;

pub static WIFI_REQUEST_CONNECT: Channel<CriticalSectionRawMutex, SetNetworkConnectionConfig, 1> = Channel::new();
pub static WIFI_REQUEST_RESCAN: Signal<CriticalSectionRawMutex, ()> = Signal::new();

async fn attempt_join_network<'a>(control: &mut cyw43::Control<'a>, config: SetNetworkConnectionConfig) -> bool {
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
            match with_timeout(
                Duration::from_secs(15),
                control.join(&config.ssid, JoinOptions::new(config.password.as_bytes())),
            )
            .await
            {
                Err(_) => {
                    info!("join timed out, attempt {} / 3", attempt);
                    continue;
                }
                Ok(Err(err)) => {
                    info!("join failed with status={}, attempt {} / 3", err.status, attempt);
                    continue;
                }
                Ok(Ok(_)) => {}
            }
        } else {
            info!("Attempting network join as open");
            match with_timeout(
                Duration::from_secs(15),
                control.join(&config.ssid, JoinOptions::new_open()),
            )
            .await
            {
                Err(_) => {
                    info!("join timed out, attempt {} / 3", attempt);
                    continue;
                }
                Ok(Err(err)) => {
                    info!("join failed with status={}, attempt {} / 3", err.status, attempt);
                    continue;
                }
                Ok(Ok(_)) => {}
            }
        }

        flash_config::save_wifi(config.clone()).await;
        update_network_bit(Some(config.ssid), BITResult::Pass).await;
        return true;
    }

    update_network_bit(Some(config.ssid), BITResult::Fail).await;
    false
}

async fn run_network_scan<'a>(control: &mut cyw43::Control<'a>) {
    for attempt in 1..=3u8 {
        info!("Running network scan...");
        // Clear previous scan
        {
            let mut configuration_state = CONFIGURATION_STATE.lock().await;
            configuration_state.available_network_connections.clear();
        }

        let scan_result = with_timeout(Duration::from_secs(15), async {
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
        })
        .await;

        match scan_result {
            Ok(_) => return,
            Err(_) => {
                if attempt < 3 {
                    info!("Network scan timed out, retrying ({} / 3)", attempt);
                } else {
                    error!("Network scan timed out, all 3 attempts failed");
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn connection_manager_task(mut control: cyw43::Control<'static>) -> ! {
    // Populate network scan state
    run_network_scan(&mut control).await;

    // Attempt to join saved networks in order (most recently connected first)
    for saved_config in flash_config::load_wifi().await {
        info!("Attempting auto-connect to {}", saved_config.ssid.as_str());
        if attempt_join_network(&mut control, saved_config).await {
            break;
        }
    }

    loop {
        match select(WIFI_REQUEST_CONNECT.receive(), WIFI_REQUEST_RESCAN.wait()).await {
            Either::First(config) => {
                info!(
                    "Got join request {}, {}",
                    config.ssid.as_str(),
                    config.password.as_str()
                );

                info!("Leaving current network (if any) before joining new one");
                control.leave().await;

                attempt_join_network(&mut control, config).await;
            }
            Either::Second(_) => {
                info!("Got network scan request");
                run_network_scan(&mut control).await
            }
        }
    }
}
