<script setup lang="ts">
import Up from "./children/Up.vue"
import UserList from "./children/UserList.vue"
import ChatArea from "./children/ChatArea.vue"
</script>

<template>
    <div class="layout">
        <Up></Up>
        <UserList></UserList>
        <ChatArea></ChatArea>
    </div>
</template>

<style scoped>
.layout {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "up up"
        "user-list chat-area";
    grid-template-rows: 60px 1fr;
    grid-template-columns: 240px 1fr;
}
</style>
