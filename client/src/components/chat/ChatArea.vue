<script setup lang="ts">
import Item from "./ChatItem.vue"
import InputArea from "./InputArea.vue"
import {reactive} from "vue";
import {Head, Msg, Type} from "../../api/backend/entity";
import storage from "../../util/storage";
import {timestamp} from "../../util/base";

let currentUserId = Number(storage.get("CURRENT_CHAT_USER"))

const msgList = reactive<Array<Msg>>([])
msgList.push(new Msg(new Head(4, Type.Text, 1, 2, timestamp(), 0, 0), "qwer"));
msgList.push(new Msg(new Head(4, Type.Text, 1, 2, timestamp(), 0, 0), "asdf"));
msgList.push(new Msg(new Head(4, Type.Text, 2, 1, timestamp(), 0, 0), "zxcv"));
msgList.push(new Msg(new Head(4, Type.Text, 2, 1, timestamp(), 0, 0), "poiu"));
msgList.push(new Msg(new Head(4, Type.Text, 1, 2, timestamp(), 0, 0), "lkjh"));
</script>

<template>
    <div class="chat-area">
        <div class="user-info"></div>
        <div class="chat-item-list">
            <div v-for="msg in msgList">
                <Item :is-sender="msg.head.sender === currentUserId" :msg="msg.payload" :timestamp="msg.head.timestamp" :type="msg.head.typ.valueOf()" :seq-num="msg.head.seq_num"></Item>
            </div>
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
