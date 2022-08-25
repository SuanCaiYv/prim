<script setup lang="ts">
import Item from "./ChatItem.vue"
import InputArea from "./InputArea.vue"
import {inject, Ref, ref, watch, watchEffect} from "vue";
import {Msg} from "../../api/backend/entity";
import {get} from "idb-keyval";
import {BASE_URL, httpClient} from "../../api/frontend";

let accountId = ref<number>(0)
let avatar = ref<string>('')
let remark = ref<string>('')
let msgChannel = inject('msgChannel') as Map<number, Array<Msg>>
let msgArray = ref(Array<Msg>())
let currentChatUserAccountId = inject('currentChatUserAccountId')
get('AccountId').then(account => {
    accountId.value = account
})
watchEffect(() => {
    // @ts-ignore
    let currentAccountId = currentChatUserAccountId.value
    httpClient.get('/friend/info/' + String(accountId.value) + '/' + String(currentAccountId), {}, true).then(async res => {
        if (res.ok) {
            // @ts-ignore
            remark.value = res.data.remark
            // @ts-ignore
            avatar.value = BASE_URL + res.data.avatar
            console.log(avatar)
        }
    })
    let arr = msgChannel.get(accountId.value)
    if (arr !== undefined) {
        msgArray.value = arr
    }
})
</script>

<template>
    <div class="chat-area">
        <div class="user-info"></div>
        <div class="chat-item-list">
            <div v-for="msg in msgArray">
                {{msg.payload}}
                <Item :avatar="avatar" :remark="remark" :type="msg.head.typ.valueOf()" :sender="msg.head.sender" :receiver="msg.head.receiver" :timestamp="msg.head.timestamp" :seq-num="msg.head.seq_num" :version="msg.head.version" :payload="msg.payload"></Item>
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
