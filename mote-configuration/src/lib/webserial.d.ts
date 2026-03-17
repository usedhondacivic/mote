// Minimal Web Serial API type augmentation.
// The full spec lives at https://wicg.github.io/serial/
// TypeScript does not yet ship these in lib.dom.d.ts.
interface Navigator {
    readonly serial: any;
}
