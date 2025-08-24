use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_usb::class::web_usb::{Config as WebUsbConfig, State, Url, WebUsb};
use embassy_usb::driver::{Driver, Endpoint, EndpointIn, EndpointOut};
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Config};
use {defmt_rtt as _, panic_probe as _};

use super::{Irqs, WebUsbConfigResources};

// This is a randomly generated GUID to allow clients on Windows to find our device
const DEVICE_INTERFACE_GUIDS: &[&str] = &["{AFB9A6FB-30BA-44BC-9232-806CFC875321}"];

pub async fn init(spawner: Spawner, r: WebUsbConfigResources) {
    // Create the driver, from the HAL.
    let driver = UsbDriver::new(r.usb, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xf569, 0x0001);
    config.manufacturer = Some("Mote");
    config.product = Some("Mote");
    config.serial_number = Some("ABCDEF");
    config.max_power = 500;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut msos_descriptor = [0; 256];

    let webusb_config = WebUsbConfig {
        max_packet_size: 64,
        vendor_code: 1,
        // If defined, shows a landing page which the device manufacturer would like the user to visit in order to control their device. Suggest the user to navigate to this URL when the device is connected.
        landing_url: Some(Url::new("https://michael-crum.com")),
    };

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    // Add the Microsoft OS Descriptor (MSOS/MOD) descriptor.
    // We tell Windows that this entire device is compatible with the "WINUSB" feature,
    // which causes it to use the built-in WinUSB driver automatically, which in turn
    // can be used by libusb/rusb software without needing a custom driver or INF file.
    // In principle you might want to call msos_feature() just on a specific function,
    // if your device also has other functions that still use standard class drivers.
    builder.msos_descriptor(windows_version::WIN8_1, 0);
    builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
    ));

    // Create classes on the builder (WebUSB just needs some setup, but doesn't return anything)
    WebUsb::configure(&mut builder, &mut state, &webusb_config);
    // Create some USB bulk endpoints for testing.
    let mut endpoints = WebEndpoints::new(&mut builder, &webusb_config);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Do some WebUSB transfers.
    let webusb_fut = async {
        loop {
            endpoints.wait_connected().await;
            info!("Connected");
            endpoints.echo().await;
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, webusb_fut).await;
}

struct WebEndpoints<'d, D: Driver<'d>> {
    write_ep: D::EndpointIn,
    read_ep: D::EndpointOut,
}

impl<'d, D: Driver<'d>> WebEndpoints<'d, D> {
    fn new(builder: &mut Builder<'d, D>, config: &'d WebUsbConfig<'d>) -> Self {
        let mut func = builder.function(0xff, 0x00, 0x00);
        let mut iface = func.interface();
        let mut alt = iface.alt_setting(0xff, 0x00, 0x00, None);

        let write_ep = alt.endpoint_bulk_in(None, config.max_packet_size);
        let read_ep = alt.endpoint_bulk_out(None, config.max_packet_size);

        WebEndpoints { write_ep, read_ep }
    }

    // Wait until the device's endpoints are enabled.
    async fn wait_connected(&mut self) {
        self.read_ep.wait_enabled().await
    }

    // Echo data back to the host.
    async fn echo(&mut self) {
        let mut buf = [0; 64];
        loop {
            let n = self.read_ep.read(&mut buf).await.unwrap();
            let data = &buf[..n];
            info!("Data read: {:x}", data);
            self.write_ep.write(data).await.unwrap();
        }
    }
}
