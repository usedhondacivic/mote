<script lang="ts">
    import ShortSpinner from "./ShortSpinner.svelte";

    let { diagnostics } = $props();

    let subsystems = $derived(
        Object.keys(diagnostics).map((key) => {
            return {
                name: key,
                tests: diagnostics[key],
            };
        }),
    );

    let result_map = {
        Waiting: ["waiting", "?"],
        Pass: ["success", "✓"],
        Fail: ["failed", "✖"],
    };
</script>

{#each subsystems as system}
    <li>
        <p style="margin: 0px;">{system.name}</p>
        <ul>
            {#if system.tests.length > 0}
                {#each system.tests as check}
                    <li>
                        <span class={result_map[check.result][0]}>
                            {#if check.result == "Waiting"}
                                <ShortSpinner />
                            {:else}
                                {result_map[check.result][1]}
                            {/if}
                        </span>
                        {check.name}
                    </li>
                {/each}
            {:else}
                <li>No checks found!</li>
            {/if}
        </ul>
    </li>
{/each}
