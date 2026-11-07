// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const { MINIBAN_HOST = "127.0.0.1", MINIBAN_PORT = "3000" } = loadEnv(mode, ".", "MINIBAN_");

  return {
    plugins: [
      sveltekit({
        compilerOptions: {
          runes: ({ filename }) => filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
        },
        adapter: adapter({ fallback: "index.html" }),
      }),
    ],
    server: {
      proxy: { "/api": `http://${MINIBAN_HOST}:${MINIBAN_PORT}` },
    },
  };
});
