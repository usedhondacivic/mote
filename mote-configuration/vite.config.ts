import { defineConfig, searchForWorkspaceRoot } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import wasm from "vite-plugin-wasm";

// https://vite.dev/config/
export default defineConfig({
    base: "/mote/configuration/",
    plugins: [svelte(), wasm()],
    server: {
        fs: {
            allow: [
                searchForWorkspaceRoot(process.cwd()), // Allow files within the workspace root
                "../"
            ],
        },
    },
    resolve: {
        preserveSymlinks: true
    },
    optimizeDeps: {
        exclude: ['mote-api']
    },
})
