import { Link } from 'mote-ffi';
import type { HostToMoteMessage, MoteToHostMessage } from './mote_api_types';

// Init WASM, init comms link
let link = new Link();

// webserial constructs (Web Serial API — types provided by the runtime environment)
let port: any;
let inputDone: any;
let outputDone: any;
let inputStream: any;
let outputStream: any;

// Result<Option<MoteToHostMessage>> as serialized by wasm-bindgen
export type PollReceiveResult = { Ok: MoteToHostMessage | null } | { Err: unknown };

export async function serial_connect(
    connect: () => void,
    disconnect: () => void,
    telemetry_recv: (data: PollReceiveResult) => void,
) {
    try {
        // See https://github.com/raspberrypi/usb-pid for vid
        const filter = { usbVendorId: 0x2e8a, usbProductId: 0x0009 };
        port = await navigator.serial.requestPort({ filters: [filter] });

        await port.open({ baudRate: 115200 });

        // Create and connect streams
        const textEncoder = new TextEncoderStream();
        outputDone = textEncoder.readable.pipeTo(port.writable);
        outputStream = textEncoder.writable.getWriter();

        // Send a zero byte (COBS delimiter) to flush any startup noise in the
        // UART buffer on the MCU side before the first real message is sent.
        await outputStream.write(new TextDecoder().decode(new Uint8Array([0])));

        connect();

        await readLoop(telemetry_recv);

        disconnect();
    } catch (error) {
        if ((error as { name: string }).name == 'NetworkError') {
            disconnect();
        }
        console.error('[serial] error:', error);
    }
}

async function readLoop(telemetry_recv: (data: PollReceiveResult) => void) {
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
        let message = Array.from(new TextEncoder().encode(value));
        link.handle_receive(message);

        // Check if one or more messages completed by the packet
        let data = link.poll_receive() as PollReceiveResult;
        while ('Ok' in data && data.Ok !== null) {
            console.log(data);
            telemetry_recv(data);
            data = link.poll_receive() as PollReceiveResult;
        }
    }
}

async function write() {
    if (!outputStream) {
        console.log("[serial] write called by serial connection is not up.");
        return;
    }

    let data = link.poll_transmit() as number[] | null;
    if (!data) {
        console.log("[serial] poll_transmit called but no data was returned.");
    }
    while (data) {
        await outputStream.write(new TextDecoder().decode(new Uint8Array(data)));
        console.log("[serial] [TX] message sent");
        data = link.poll_transmit() as number[] | null;
    }
}

// UI event handlers
export async function set_uid(uid: string, error_handler: () => void) {
    if (uid.length > 3) {
        const msg: HostToMoteMessage = { SetUID: { uid } };
        link.send(msg);
        await write();
    } else {
        error_handler()
    }
}

export async function network_connect(ssid: string, password: string) {
    const msg: HostToMoteMessage = { SetNetworkConnectionConfig: { ssid, password } };
    link.send(msg);
    await write();
}

export async function rescan() {
    const msg: HostToMoteMessage = "RequestNetworkScan";
    link.send(msg);
    await write();
}
