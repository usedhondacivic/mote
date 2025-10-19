<script lang="ts">
    import { network_connect } from "./mote_api";

    let { entry_data, is_current_connection } = $props();

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
            network_connect(entry_data?.ssid, "test");
            input_open = false;
        }
    }
</script>

<li>
    <span style="margin: 0px;"> {entry_data?.ssid} </span>
    <span style="float: right; margin: 0px">
        {#if is_current_connection}
            &lt;- currently connected
        {:else}
            <pre>{get_indicator(entry_data?.strength)}</pre>
            |<button
                id={entry_data?.ssid}
                onclick={() => {
                    if (input_open) {
                        network_connect(entry_data?.ssid, "test");
                        input_open = false;
                    } else {
                        input_open = true;
                    }
                }}>[ connect ]</button
            >
        {/if}
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
</li>
