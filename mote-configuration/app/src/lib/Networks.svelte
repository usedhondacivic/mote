<script lang="ts">
    import NetworkEntry from "./NetworkEntry.svelte";

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
    <NetworkEntry
        ssid={network?.ssid.replace(/\0/g, "").trim()}
        strength={network?.strength}
        is_current_connection={network?.ssid.replace(/\0/g, "").trim() ==
            current_connection}
    />
{/each}
