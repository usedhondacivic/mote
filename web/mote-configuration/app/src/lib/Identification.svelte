<script lang="ts">
    import { tick } from "svelte";
    import ShortSpinner from "./ShortSpinner.svelte";

    import { set_uid } from "./mote_api";

    let { uid, ip } = $props();

    let input_open = $state(false);
    let input_value = $state("");
    let input_ref: HTMLElement;

    function submit() {
        set_uid(input_value, () => {
            console.log("set uid error");
        });
        input_open = false;
    }

    function handle_key(event: KeyboardEvent) {
        if (event.repeat) return;

        if (event.key === "Enter") {
            submit();
        }
    }
</script>

<li>
    Unique ID: {uid}
    <span style="float: right; margin: 0px;">
        <button
            onclick={async () => {
                if (input_open) {
                    set_uid(input_value, () => {
                        console.log("set uid error");
                    });
                    input_open = false;
                } else {
                    input_open = true;
                    await tick();
                    input_ref.focus();
                }
            }}>[ update ]</button
        ></span
    >
    <ul hidden={!input_open}>
        <li>
            <input
                type="text"
                id="uid"
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
<li>
    IP:
    {#if ip}
        ip
    {:else}<ShortSpinner />{/if}
</li>
