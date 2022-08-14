<script setup lang="ts">
import Item from "../../chat/ChatItem.vue"
import InputArea from "../../chat/InputArea.vue"
</script>

<template>
    <div class="chat-area">
        <div class="user-info"></div>
        <div class="chat-item-list">
            <Item :is-sender="true"></Item>
            <Item :is-sender="true"></Item>
            <Item :is-sender="false"></Item>
            <Item :is-sender="true"></Item>
            <Item :is-sender="false"></Item>
            <Item :is-sender="true"></Item>
            <Item :is-sender="true"></Item>
            <Item :is-sender="false"></Item>
            <Item :is-sender="true"></Item>
        </div>
        <InputArea></InputArea>
    </div>
</template>

<style scoped>
.chat-area {
    grid-area: chat-area;
    background-color: white;
    overflow-y: scroll;
    display: grid;
    grid-template-areas:
        "user-info"
        "chat-item-list"
        "input-area";
    grid-template-rows: 40px 1fr 180px;
}

.user-info {
    grid-area: user-info;
    height: 40px;
    width: 100px;
}

.chat-item-list {
    grid-area: chat-item-list;
    width: 100%;
    overflow-y: scroll;
}

.input-area {
    grid-area: input-area;
    width: 100%;
}

::-webkit-scrollbar {
    display: none;
}
</style>
