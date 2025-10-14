<script lang="ts">
    import ShortSpinner from "./ShortSpinner.svelte";

    import { set_uid } from "./mote_api";

    let { uid, ip } = $props();

    let input_open = $state(false);
    let input_value = $state("");

    function handle_key(event: KeyboardEvent) {
        if (event.repeat) return;

        if (event.key === "Enter") {
            set_uid(input_value, () => {
                console.log("set uid error");
            });
        }
    }
</script>

<li>
    Unique ID: {uid}
    <span style="float: right; margin: 0px;">
        <button
            onclick={() => {
                if (input_open) {
                    set_uid(input_value, () => {
                        console.log("set uid error");
                    });
                }
                input_open = true;
            }}>[ update ]</button
        ></span
    >
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
<li>
    IP:
    {#if ip}
        ip
    {:else}<ShortSpinner />{/if}
</li>
