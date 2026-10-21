<script setup lang="ts">
const { public: config } = useRuntimeConfig()
// Client-side only: in Docker the server-side render cannot reach the backend on localhost.
const { data, error } = useFetch<{ status: string }>(`${config.apiBase}/health`, {
  server: false
})
</script>

<template>
  <main>
    <h1>It works</h1>
    <p v-if="data">Backend: {{ data.status }}</p>
    <p v-else-if="error">Backend not reachable at {{ config.apiBase }}</p>
    <p v-else>Checking backend...</p>
  </main>
</template>
