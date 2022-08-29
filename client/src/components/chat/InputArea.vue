<script setup lang="ts">
import {ref} from "vue";
import {sendMsgChannel, withId} from "../../system/net";
import {Head, Msg, Type} from "../../api/backend/entity";
import {get} from "idb-keyval";
import {timestamp} from "../../util/base";

let text = ref<string>('')

const send = async () => {
    console.log('sent')
    if (text.value.endsWith('\n')) {
        text.value = text.value.substring(0, text.value.length - 1)
    }
    if (text.value === '') {
        return
    }
    const accountId = await get('AccountId')
    const head = new Head(text.value.length, Type.Text, Number(accountId), Number(withId.value), timestamp(), 0, 0);
    sendMsgChannel.push(new Msg(head, text.value))
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
