use defmt::{info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, Instance as UsbInstance};
use embassy_time::{Duration, Ticker, with_timeout};
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use mote_messages::configuration::{host_to_mote, mote_to_host};
use postcard::{take_from_bytes_cobs, to_slice_cobs};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Irqs, UsbSerialResources};
use crate::tasks::CONFIGURATION_STATE;

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn handle_serial<'d, T: UsbInstance + 'd>(
    class: &mut CdcAcmClass<'d, UsbDriver<'d, T>>,
) -> Result<(), Disconnected> {
    // TODO: postcard serialized size is currently experimental and not implemented
    // as a constant fn. After that feature is stabilized, consider how to rightsize
    // this buffer
    let mut serialization_buffer = heapless::Vec::<u8, 256>::new();
    let mut serial_buffer = [0; 64];

    // Periodic timer for telemetering state
    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        match select(class.read_packet(&mut serial_buffer), ticker.next()).await {
            Either::First(Ok(bytes_read)) => {
                serialization_buffer
                    .extend_from_slice(&serial_buffer[..bytes_read])
                    .unwrap();
                while let Some(end) = serialization_buffer.iter().position(|&x| x == 0) {
                    let mut idx = 0;
                    loop {
                        match take_from_bytes_cobs::<host_to_mote::Message>(&mut serialization_buffer[idx..end + 1]) {
                            Ok((msg, remainder)) => {
                                info!("Got: {:?}", msg);
                                serialization_buffer = heapless::Vec::from_slice(remainder).unwrap();
                                break;
                            }
                            Err(postcard::Error::DeserializeBadEncoding) => {
                                idx += 1;
                            }
                            Err(err) => {
                                warn!("{:x}", err);
                                serialization_buffer.clear();
                                break;
                            }
                        }
                    }
                }
                Ok(())
            }
            Either::First(Err(error)) => Err(error),
            Either::Second(_) => {
                if let Ok(configuration_state) =
                    with_timeout(Duration::from_millis(500), CONFIGURATION_STATE.lock()).await
                {
                    let message = mote_to_host::Message::State(configuration_state.clone());
                    let packet = to_slice_cobs(&message, &mut serial_buffer).unwrap();
                    info!("Sending {:x}", packet);
                    info!("Sending {:?}", *configuration_state);
                    if let Ok(res) = with_timeout(Duration::from_millis(500), class.write_packet(&packet)).await {
                        res?;
                    }
                }
                Ok(())
            }
        }?;
    }
}

#[embassy_executor::task]
async fn usb_serial_task(spawner: Spawner, r: UsbSerialResources) {
    let driver = UsbDriver::new(r.usb, Irqs);

    // See https://github.com/raspberrypi/usb-pid for vid / pid
    let config = {
        let mut config = embassy_usb::Config::new(0x2E8A, 0x0009); // rpi CDC UART
        config.manufacturer = Some("Mote");
        config.product = Some("Mote Serial");
        config.serial_number = Some("12345678"); // TODO: fill this in
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    unwrap!(spawner.spawn(usb_task(usb)));

    loop {
        class.wait_connection().await;
        info!("USB serial connected");
        let _ = handle_serial(&mut class).await;
        info!("USB serial disconnected");
    }
}

pub async fn init(spawner: Spawner, r: UsbSerialResources) {
    unwrap!(spawner.spawn(usb_serial_task(spawner, r)));
}
