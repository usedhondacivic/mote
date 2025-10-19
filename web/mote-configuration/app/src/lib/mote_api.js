import init, { ConfigurationLink } from '../../../pkg/configuration.js';

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

        // Parse message
        link.handle_receive(new TextEncoder().encode(value));

        // Check if one or more messages completed by the packet
        let data = link.poll_receive();
        while (data?.Ok) {
            telemetry_recv(data);
            data = link.poll_receive();
        }
    }
}

async function write() {
    if (!outputStream) {
        console.log("[serial] write called by serial connection is not up.");
    }

    let data = link.poll_transmit();
    if (!data) {
        console.log("[serial] poll_transmit called but no data was returned.");
    }
    while (data) {
        await outputStream.write(new TextDecoder().decode(new Uint8Array(data)));
        console.log("[serial] [TX] message sent");
        data = link.poll_transmit();
    }
}

// UI event handlers
export async function set_uid(uid, error_handler) {
    if (uid.length > 3) {
        link.send(
            {
                SetUID: {
                    uid: uid
                }
            });
        await write();
    } else {
        error_handler()
    }
}

export async function network_connect(ssid, password) {
    link.send({
        SetNetworkConnectionConfig: {
            ssid: ssid,
            password: password
        }
    });
    await write();
}

export async function rescan() {
    link.send("RequestNetworkScan");
    await write();
}

