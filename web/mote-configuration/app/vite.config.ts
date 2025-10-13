import { defineConfig, searchForWorkspaceRoot } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig({
    base: "/mote/configuration",
    plugins: [svelte()],
    server: {
        fs: {
            allow: [
                searchForWorkspaceRoot(process.cwd()), // Allow files within the workspace root
                "../"
            ],
        },
    },
})
