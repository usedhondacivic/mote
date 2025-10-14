<script lang="ts">
    import { mote_telem } from "./shared.svelte";

    let { networks, current_connection } = $props();

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

    let sorted_networks = $derived.by(() => {
        if (networks) {
            return networks.toSorted((a: any, b: any) => {
                if (current_connection) {
                    if (
                        a?.ssid.replace(/\0/g, "").trim() ==
                        current_connection.replace(/\0/g, "").trim()
                    ) {
                        return -1;
                    }
                    if (
                        b?.ssid.replace(/\0/g, "").trim() ==
                        current_connection.replace(/\0/g, "").trim()
                    ) {
                        return 1;
                    }
                }
                return a?.strength - b?.strength;
            });
        } else {
            return [];
        }
    });
</script>

{#each sorted_networks as network}
    <li>
        <span style="margin: 0px;"> {network?.ssid} </span>
        <span style="float: right; margin: 0px">
            {#if network?.ssid.replace(/\0/g, "").trim() == current_connection
                    .replace(/\0/g, "")
                    .trim()}
                &lt;- currently connected
            {:else}
                <pre>{get_indicator(network?.strength)}</pre>
                |<button id={network?.ssid}>[ connect ]</button>
            {/if}
        </span>
    </li>
{/each}
