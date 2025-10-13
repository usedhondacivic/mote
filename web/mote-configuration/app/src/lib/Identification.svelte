<script lang="ts">
    import ShortSpinner from "./ShortSpinner.svelte";

    let { uid, ip } = $props();
</script>

<li>
    Unique ID: {uid}
    <span style="float: right; margin: 0px;"> <button>[ update ]</button></span>
    <input
        type="text"
        id="uid"
        name="uid"
        placeholder="enter new UID"
        autocomplete="off"
    />
</li>
<li>
    IP:
    {#if ip}
        ip
    {:else}<ShortSpinner />{/if}
</li>
