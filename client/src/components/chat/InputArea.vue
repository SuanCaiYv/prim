<script setup lang="ts">
import {inject, ref} from "vue";
import {Ref} from "_vue@3.2.37@vue";
import {KV} from "../../api/frontend/interface";

let currentChatUserAccountId = inject('currentChatUserAccountId') as Ref<number>
let sendMsgChannel = inject('sendMsgChannel') as Array<KV<string, number>>

let text = ref<string>('')

const send = () => {
    console.log('sent')
    if (text.value === '') {
        return
    }
    sendMsgChannel.push(new KV(text.value, currentChatUserAccountId.value))
    text.value = ''
}
</script>

<template>
    <div class="input-area">
        <textarea class="input" @keyup.enter="send" v-model="text"></textarea>
    </div>
</template>

<style scoped>
.input-area {
    grid-area: input-area;
    background-color: white;
    height: 100%;
    width: 100%;
    overflow-y: scroll;
}

.input {
    width: 100%;
    height: 100%;
    border: none;
    padding: 12px 0 0 12px;
    outline: none;
    resize: none;
    box-sizing: border-box;
    border-top: 1px solid gainsboro;
    font-size: 1rem;
    color: black;
    background-color: white;
}

::-webkit-scrollbar {
    display: none;
}
</style>
