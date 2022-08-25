<script setup lang="ts">
import Up from "../base/children/Up.vue"
import UserList from "./UserList.vue"
import ChatArea from "../chat/ChatArea.vue"
import {provide, ref} from "vue";

let showChatArea = ref<boolean>(false)
const showChatAreaFunc = () => {
    showChatArea.value = false
    showChatArea.value = true
}
provide('showChatAreaFunc', showChatAreaFunc)
</script>

<template>
    <div class="layout">
        <Up></Up>
        <UserList></UserList>
        <ChatArea v-if="showChatArea"></ChatArea>
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
