<script setup lang="ts">

import {watchEffect} from "vue";
import {chatList, chatSet, msgListMap, msgListMapKey, netApi, put_suitable, sendMsgChannel} from "./system/net";
import {get} from "idb-keyval";
import {watch} from "vue";
import {Msg} from "./api/backend/entity";

watchEffect(async () => {
    if (sendMsgChannel.length > 0) {
        const len = sendMsgChannel.length;
        if (netApi !== undefined) {
            console.log('sent')
            const msg = sendMsgChannel[len - 1];
            await netApi.send_msg(msg)
            let m = msgListMap.get(await msgListMapKey(msg.head.sender, msg.head.receiver))
            if (m === undefined) {
                m = new Array<Msg>()
                msgListMap.set(await msgListMapKey(msg.head.sender, msg.head.receiver), m)
            }
            await put_suitable(msg, m)
            sendMsgChannel.splice(len - 1, 1)
        }
    }
    console.log('send msg channel', sendMsgChannel)
})

watch(chatSet, async (chatSet, o) => {
    // console.log('chat set', chatSet)
    let map = chatSet.get((await get('AccountId')) as number)
    if (map === undefined) {
        map = new Map<number, number>()
        chatSet.set((await get('AccountId')) as number, map)
    }
    let arr = new Array<Map<number, number>>();
    let entries = map.entries()
    for (let [key, value] of entries) {
        let m = new Map<number, number>()
        m.set(key, value)
        // console.log('entry', m)
        arr.push(m)
    }
    chatList.splice(0, chatList.length, ...arr)
    // console.log('chat list', chatList)
})
</script>

<template>
    <router-view>
    </router-view>
</template>

<style scoped>
#app {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-align: center;
    font-size: 20px;
    height: 100%;
    width: 100%;
}
</style>
