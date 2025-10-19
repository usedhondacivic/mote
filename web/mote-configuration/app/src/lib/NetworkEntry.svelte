<script lang="ts">
    import { network_connect } from "./mote_api";

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

    function handle_key(event: KeyboardEvent) {
        if (event.repeat) return;

        if (event.key === "Enter") {
            network_connect(ssid, input_value);
            input_open = false;
        }
    }
</script>

<li class:success={is_current_connection}>
    <span style="margin: 0px;">
        {ssid}
    </span>
    {#if is_current_connection}
        <span style="float: right; margin: 0px">&lt;~~ currently connected</span
        >
    {:else}
        <span style="float: right; margin: 0px">
            <pre>{get_indicator(strength)}</pre>
            |<button
                id={ssid}
                onclick={() => {
                    if (input_open) {
                        network_connect(ssid, "test");
                        input_open = false;
                    } else {
                        input_open = true;
                    }
                }}>[ connect ]</button
            >
        </span>
        {#if input_open}
            <ul>
                <li>
                    <input
                        type="text"
                        id="uid"
                        name="uid"
                        placeholder="enter new UID"
                        autocomplete="off"
                        bind:value={input_value}
                        onkeydown={handle_key}
                    />
                </li>
            </ul>
        {/if}
    {/if}
</li>
