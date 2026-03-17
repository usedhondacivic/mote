import type { MoteToHostMessage, State } from './mote_api_types';

type PollReceiveResult = { Ok: MoteToHostMessage | null } | { Err: unknown };

interface MoteTelem {
    latest: Partial<State>
}

export let mote_telem: MoteTelem = $state({ latest: {} });


export function handle_telem_recv(telem: PollReceiveResult) {
    if ('Ok' in telem && telem.Ok !== null && typeof telem.Ok === 'object' && 'State' in telem.Ok) {
        Object.assign(mote_telem.latest, telem.Ok.State);
    }
}

const long_spinner_characters = ["⢀⠀", "⡀⠀", "⠄⠀", "⢂⠀", "⡂⠀", "⠅⠀", "⢃⠀", "⡃⠀", "⠍⠀", "⢋⠀", "⡋⠀", "⠍⠁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉", "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⢈⠩", "⡀⢙", "⠄⡙", "⢂⠩", "⡂⢘", "⠅⡘", "⢃⠨", "⡃⢐", "⠍⡐", "⢋⠠", "⡋⢀", "⠍⡁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉", "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⠈⠩", "⠀⢙", "⠀⡙", "⠀⠩", "⠀⢘", "⠀⡘", "⠀⠨", "⠀⢐", "⠀⡐", "⠀⠠", "⠀⢀", "⠀⡀"];
const short_spinner_characters = ["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"];

export const long_spinner_state = $state({
    character: long_spinner_characters[0],
    count: 0
});

export const short_spinner_state = $state({
    character: short_spinner_characters[0],
    count: 0
});

$effect.root(() => {
    const interval = setInterval(() => {
        long_spinner_state.count = (long_spinner_state.count + 1) % long_spinner_characters.length;
        long_spinner_state.character = long_spinner_characters[long_spinner_state.count];

        short_spinner_state.count = (short_spinner_state.count + 1) % short_spinner_characters.length;
        short_spinner_state.character = short_spinner_characters[short_spinner_state.count];
    }, 100);
    return () => {
        clearInterval(interval);
    };
}
)
