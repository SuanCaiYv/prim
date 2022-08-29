<script setup lang="ts">
import Item from "./ChatItem.vue"
import InputArea from "./InputArea.vue"
import {computed, ref, watch, watchEffect} from "vue";
import {Msg, SyncArgs} from "../../api/backend/entity";
import {get} from "idb-keyval";
import {BASE_URL, httpClient} from "../../api/frontend";
import {useStore} from "vuex";
import {msgListMap, msgListMapKey, netApi, syncMsgDone, syncMsgOldest, syncMsgRepeat, withId} from "../../system/net";

let accountId = ref<number>(0)
let withAvatar = ref<string>('')
let withRemark = ref<string>('')
let msgArray = ref(Array<Msg>())

get('AccountId').then(account => {
    accountId.value = account
})

// console.log('msg list map', msgListMap)

watchEffect(() => {
    const withAccountId = withId.value;
    console.log(withAccountId)
})

watch(withId, async (n, o) => {
    console.log('with id: ', n)
    // @ts-ignore
    let currentAccountId = Number(n.value);
    httpClient.get('/friend/info/' + String(accountId.value) + '/' + String(currentAccountId), {}, true).then(async res => {
        if (res.ok) {
            // @ts-ignore
            withRemark.value = res.data.remark
            // @ts-ignore
            withAvatar.value = BASE_URL + res.data.avatar
        }
    })
    let arr = msgListMap.get(await msgListMapKey(currentAccountId))
    if (arr === undefined) {
        arr = new Array<Msg>()
        msgListMap.set(await msgListMapKey(currentAccountId), arr)
    }
    console.log('with-id change')
    while (!Boolean(syncMsgRepeat.get(await msgListMapKey(currentAccountId))) && !Boolean(syncMsgDone.get(await msgListMapKey(currentAccountId)))) {
        console.log('sync', currentAccountId)
        await netApi.send_msg(await Msg.sync(new SyncArgs(Number(syncMsgOldest.get(await msgListMapKey(currentAccountId))), true, 20), currentAccountId))
    }
    msgArray.value = arr
})

watch(msgListMap, async (n, o) => {
    let arr = msgListMap.get(await msgListMapKey(Number(withId.value)))
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
                <Item :avatar="withAvatar" :remark="withRemark" :type="msg.head.typ.valueOf()" :sender="msg.head.sender" :receiver="msg.head.receiver" :timestamp="msg.head.timestamp" :seq-num="msg.head.seq_num" :version="msg.head.version" :payload="msg.payload"></Item>
            </div>
        </div>
        <Suspense>
            <InputArea></InputArea>
        </Suspense>
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
