<!-- Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details) -->

<!-- @component The placeholder page: the product name, a description, and the server's status. -->

<script lang="ts">
import { onMount } from "svelte";

type Status = "checking" | "ok" | "unavailable";

const labels: Record<Status, string> = {
  checking: "Checking server...",
  ok: "Server is healthy",
  unavailable: "Server is unavailable",
};

let status: Status = $state("checking");

/** Reads the server status from the health API on the page's own origin. */
async function fetchStatus(): Promise<Status> {
  try {
    const response = await fetch("/api/health");
    if (!response.ok) return "unavailable";
    const body = await response.json();
    return body.status === "ok" ? "ok" : "unavailable";
  } catch {
    return "unavailable";
  }
}

onMount(async () => {
  status = await fetchStatus();
});
</script>

<main>
  <h1>Miniban</h1>
  <p>An opinionated work model layered over existing issue trackers.</p>
  <p class="status" data-status={status} role="status">{labels[status]}</p>
</main>

<style>
main {
  max-width: 40rem;
  margin: 4rem auto;
  padding: 0 1rem;
  font-family: system-ui, sans-serif;
}

.status::before {
  content: "";
  display: inline-block;
  width: 0.6rem;
  height: 0.6rem;
  margin-right: 0.5rem;
  border-radius: 50%;
  background: gray;
}

.status[data-status="ok"]::before {
  background: green;
}

.status[data-status="unavailable"]::before {
  background: firebrick;
}
</style>
