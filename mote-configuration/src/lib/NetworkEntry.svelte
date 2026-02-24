<script lang="ts">
    import { tick } from "svelte";
    import { network_connect } from "./link";

    let { ssid, strength, is_current_connection } = $props();

    const wifi_strength_indicators = ["[••••]", "[••• ]", "[••  ]", "[•   ]"];

    function get_indicator(strength: number) {
        return wifi_strength_indicators[
            Math.max(
                Math.min(
                    Math.floor(strength / 20) - 1,
                    wifi_strength_indicators.length - 1,
                ),
                0,
            )
        ];
    }

    let input_open = $state(false);
    let input_value = $state("");
    let input_ref;

    function submit() {
        network_connect(ssid, input_value);
        input_open = false;
    }

    function handle_key(event: KeyboardEvent) {
        if (event.repeat) return;

        if (event.key === "Enter") {
            submit();
        }
    }
</script>

<li class:success={is_current_connection}>
    <span style="margin: 0px;">
        {ssid}
    </span>
    <span style="float: right; margin: 0px" hidden={!is_current_connection}
        >&lt;~~ currently connected</span
    >
    <span style="float: right; margin: 0px" hidden={is_current_connection}>
        <pre>{get_indicator(strength)}</pre>
        |<button
            id={ssid}
            onclick={async () => {
                if (input_open) {
                    submit();
                } else {
                    input_open = true;
                    await tick();
                    input_ref.focus();
                }
            }}>[ connect ]</button
        >
    </span>
    <ul hidden={!input_open}>
        <li>
            <input
                type="text"
                name="uid"
                placeholder="enter new UID"
                autocomplete="off"
                bind:this={input_ref}
                bind:value={input_value}
                onkeydown={handle_key}
            />
        </li>
    </ul>
</li>
