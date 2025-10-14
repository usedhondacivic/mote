<script lang="ts">
    import { handle_telem_recv, mote_telem } from "./lib/shared.svelte";
    import { rescan, serial_connect } from "./lib/mote_api";

    import LongSpinner from "./lib/LongSpinner.svelte";

    import Identification from "./lib/Identification.svelte";
    import Networks from "./lib/Networks.svelte";
    import Diagnostics from "./lib/Diagnostics.svelte";

    let serial_connection = $state({
        connected: false,
        has_received: false,
        time_since_received: 0,
        last_telem_time: new Date(),
    });

    $effect(() => {
        const interval = setInterval(() => {
            serial_connection.time_since_received =
                new Date().getTime() -
                serial_connection.last_telem_time.getTime();
        }, 100);
        return () => {
            clearInterval(interval);
        };
    });
</script>

<main>
    <div style="margin-top: 10ch;" class="tree">
        <ul>
            <p style="margin: 0px;"><strong>Mote</strong></p>
            <li class="preconnection">
                Serial: <span
                    class={serial_connection.connected ? "success" : "failed"}
                    >{serial_connection.connected
                        ? "connected"
                        : "disconnected"}</span
                >
                {#if !serial_connection.connected}
                    <button
                        style="float: right;"
                        onclick={() => {
                            serial_connect(
                                (_: Event) => {
                                    serial_connection.connected = true;
                                },
                                (_: Event) => {
                                    serial_connection.connected = false;
                                },
                                (telem: Object) => {
                                    serial_connection.last_telem_time =
                                        new Date();
                                    serial_connection.has_received = true;
                                    handle_telem_recv(telem);
                                },
                            );
                        }}>[ Connect ]</button
                    >
                {/if}
            </li>
            <li>
                Telemetry last received: <span
                    class={serial_connection.connected ? "success" : "failed"}
                    >{serial_connection.connected
                        ? (
                              serial_connection.time_since_received / 1000
                          ).toFixed(2) + "s ago"
                        : "never"}</span
                >
            </li>
            <li>
                <p style="margin: 0px;"><strong>Identification</strong></p>
                <ul>
                    {#if mote_telem.latest?.uid}
                        <Identification
                            uid={mote_telem.latest?.uid}
                            ip={mote_telem.latest?.ip}
                        />
                    {:else}
                        <li>
                            <LongSpinner />
                        </li>
                    {/if}
                </ul>
            </li>
            <li>
                <p style="margin: 0px;">
                    <strong>Detected Networks</strong>
                    {#if mote_telem.latest?.uid}
                        <button
                            style="float: right; margin: 0px"
                            onclick={rescan}
                        >
                            [ refresh ]
                        </button>
                    {/if}
                </p>
                <ul>
                    {#if mote_telem.latest?.available_network_connections?.length > 0}
                        <Networks
                            networks={mote_telem.latest
                                ?.available_network_connections}
                            current_connection={mote_telem.latest
                                ?.current_network_connection}
                        />
                    {:else if mote_telem.latest?.available_network_connections}
                        <li>No networks available</li>
                    {:else}
                        <li><LongSpinner /></li>
                    {/if}
                </ul>
            </li>
            <li>
                <p style="margin: 0px;"><strong>Diagnositics</strong></p>
                <ul>
                    {#if mote_telem.latest?.built_in_test}
                        <Diagnostics
                            diagnostics={mote_telem.latest?.built_in_test}
                        />
                    {:else}
                        <li><LongSpinner /></li>
                    {/if}
                </ul>
            </li>
        </ul>
    </div>
</main>

<style>
    :global(.success) {
        color: lightgreen;
    }
    :global(.waiting) {
        color: #fdfd96;
    }
    :global(.failed) {
        color: salmon;
    }
</style>
