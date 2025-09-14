import init, { ConfigurationLink } from '../pkg/configuration.js';

// Init WASM, init comms link
await init();
let link = new ConfigurationLink();

// webserial constructs
let port;
let inputDone;
let outputDone;
let inputStream;
let outputStream;

export async function serial_connect(connect, disconnect, telemetry_recv) {
    try {
        // See https://github.com/raspberrypi/usb-pid for vid
        const filter = { usbVendorId: 0x2e8a, usbProductId: 0x0009 };
        port = await navigator.serial.requestPort({ filters: [filter] });

        await port.open({ baudRate: 115200 });

        connect();

        // Create and connect streams
        const textEncoder = new TextEncoderStream();
        outputDone = textEncoder.readable.pipeTo(port.writable);
        outputStream = textEncoder.writable.getWriter();

        await readLoop(telemetry_recv);

        disconnect();
    } catch (error) {
        if (error.name == 'NetworkError') {
            disconnect();
        }
        console.error('[serial] error:', error);
    }
}

async function readLoop(telemetry_recv) {
    console.log("[serial] start read loop");
    const textDecoder = new TextDecoderStream();
    inputDone = port.readable.pipeTo(textDecoder.writable);
    inputStream = textDecoder.readable.getReader();

    while (true) {
        const { value, done } = await inputStream.read();
        if (done) {
            console.log('[serial] Input DONE');
            inputStream.releaseLock();
            break;
        }
        telemetry_recv(link.handle_receive(new TextEncoder().encode(value)));
    }
}

async function write() {
    if (!outputStream) {
        console.log("[serial] write called by serial connection is not up.");
    }
    let data = link.poll_transmit();
    if (data) {
        await outputStream.write(new TextDecoder().decode(new Uint8Array(data)));
        console.log("[serial] [TX] message sent");
    } else {
        console.log("[serial] poll_transmit called but no data was returned.");
    }
}

// UI event handlers
export async function set_uid(uid) {
    link.send(
        {
            SetUID: {
                uid: uid
            }
        });
    await write();
}

export function select_ssid() {
    console.log("select_ssid");
}

export function network_connect() {
    console.log("network_connect");
}

export function rescan() {
    link.send("RequestNetworkScan");
}

export function poll_transmit() {

}

export function handle_receive() {

}
