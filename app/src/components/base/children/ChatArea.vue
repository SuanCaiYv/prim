<script setup lang="ts">
import Item from "../../chat/ChatItem.vue"
</script>

<template>
    <div class="chat-area">
        <div class="user-info"></div>
        <Item :is-sender="true"></Item>
        <Item :is-sender="true"></Item>
        <Item :is-sender="false"></Item>
        <Item :is-sender="true"></Item>
    </div>
</template>

<style scoped>
.chat-area {
    grid-area: chat-area;
    background-color: beige;
    overflow-y: scroll;
}

.user-info {
    height: 40px;
    width: 100px;
}

::-webkit-scrollbar {
    display: none;
}
</style>
