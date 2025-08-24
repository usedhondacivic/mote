use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, Instance as UsbInstance};
use embassy_time::{Duration, Ticker};
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use mote_messages::configuration::host_to_mote;
use postcard::from_bytes;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Irqs, UsbSerialResources};

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

fn write_state() {}

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
    let mut message_aggegator = heapless::HistoryBuffer::<u8, 256>::new();
    let mut serialization_buffer = heapless::Vec::<u8, 256>::new();
    let mut serial_buffer = [0; 64];

    // Periodic timer for telemetering state
    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        match select(class.read_packet(&mut serial_buffer), ticker.next()).await {
            Either::First(Ok(bytes_read)) => {
                message_aggegator.extend(&serial_buffer[..bytes_read]);
                serialization_buffer.clear();
                serialization_buffer
                    .extend_from_slice(message_aggegator.as_slices().0)
                    .unwrap();
                serialization_buffer
                    .extend_from_slice(message_aggegator.as_slices().1)
                    .unwrap();
                match from_bytes(&serialization_buffer) {
                    Ok(host_to_mote::Message::SetUID(uid)) => {
                        info!("Recieved SetUID request: {:?}", uid);
                    }
                    Ok(host_to_mote::Message::SetNetworkConnectionConfig(config)) => {
                        info!("Recieved SetNetworkconfig request: {:?}", config);
                    }
                    _ => (),
                }
                Ok(())
            }
            Either::First(Err(error)) => Err(error),
            Either::Second(_) => Ok(()),
        }?;

        let n = class.read_packet(&mut serial_buffer).await?;
        let data = &serial_buffer[..n];
        info!("data: {:x}", data);
        class.write_packet(data).await?;
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
