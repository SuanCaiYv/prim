<script setup lang="ts">
import Item from "./ChatItem.vue"
import InputArea from "./InputArea.vue"
import {inject, ref} from "vue";
import {Msg} from "../../api/backend/entity";
import {get} from "idb-keyval";
import {httpClient} from "../../api/frontend";

let accountId = ref<number>(0)
let currentUserId = ref<number>(0);
let avatar = ref<string>('')
let remark = ref<string>('')
get("CurrentChatUserAccountId").then(accountId => {
    currentUserId.value = accountId;
})
get('AccountId').then(account => {
    accountId.value = account
})
let msgChannel = inject('msgChannel') as Map<number, Array<Msg>>
let msgArray = msgChannel.get(currentUserId.value)

httpClient.get('/friend/info/' + String(accountId.value) + String(currentUserId.value), {}, true).then(async res => {
    if (res.ok) {
        // @ts-ignore
        remark.value = res.data.remark
        // @ts-ignore
        avatar.value = res.data.avatar
    }
})
</script>

<template>
    <div class="chat-area">
        <div class="user-info"></div>
        <div class="chat-item-list">
            <div v-for="msg in msgArray">
                <Item :avatar="avatar" :remark="remark" :is-sender="msg.head.sender === currentUserId" :msg="msg.payload" :timestamp="msg.head.timestamp" :type="msg.head.typ.valueOf()" :seq-num="msg.head.seq_num"></Item>
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
