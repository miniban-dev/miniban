// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

/**
 * Vite configuration: builds the SvelteKit SPA and proxies `/api` to the local server in development.
 *
 * @module
 */

import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

/** The runtime's environment. Declared here because the project has no Node.js type definitions. */
declare const process: { env: Record<string, string | undefined> };

const { MINIBAN_HOST = "127.0.0.1", MINIBAN_PORT = "3000" } = process.env;

/** A URL needs an IPv6 address in brackets, and only an IPv6 address contains a colon. */
const proxyHost = MINIBAN_HOST.includes(":") ? `[${MINIBAN_HOST}]` : MINIBAN_HOST;

/** The Vite configuration, whose proxy targets the server the process environment describes. */
export default defineConfig({
  plugins: [
    sveltekit({
      compilerOptions: {
        runes: ({ filename }) => filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
      },
      adapter: adapter({ fallback: "index.html" }),
    }),
  ],
  server: {
    proxy: { "/api": `http://${proxyHost}:${MINIBAN_PORT}` },
  },
});
