<script setup lang="ts">
import Up from "./children/Up.vue"</script>

<template>
    <div class="layout">
        <Up></Up>
        <slot name="user-list"></slot>
        <slot name="chat-area"></slot>
    </div>
</template>

<style scoped>
.layout {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "up up"
        "list chat-area";
    grid-template-rows: 60px 1fr;
    grid-template-columns: 240px 1fr;
}
</style>
