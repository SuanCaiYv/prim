<script setup lang="ts">

import {useRouter} from "vue-router";
import {reactive, ref} from "vue";
import {get} from "_idb-keyval@6.2.0@idb-keyval";
import {Constant} from "../../system/constant";
import {BASE_URL, httpClient} from "../../api/frontend/http";
import {addFriend, addFunc, createGroup} from "../common/jetcomponents";
import alertFunc from "../alert/alert";
import {msgChannelMap, sendMsgChannel, userMsgList, withAccountId} from "../../function/types";
import UserMsgItem from './UserMsgItem.vue'
import ChatItem from './ChatItem.vue'
import {watch} from "_vue@3.2.37@vue";
import {Head, Msg, Type} from "../../api/backend/entity";
import {msgChannelMapKey, startNet, tryClosePreviousNet} from "../../function/net";
import {timestamp} from "../../util/base";

const router = useRouter()
let avatar = ref<string>('')
let accountId = ref<number>(0)
let msgArray = reactive<Array<Msg>>(new Array<Msg>())
let withAvatar = ref<string>(BASE_URL + '/static/default-avatar.png')
let withRemark = ref<string>('')
let text = ref<string>('')

get(Constant.Authed).then(authed => {
    if (!authed) {
        router.push('/sign')
    } else {
        startNet()
    }
})

get(Constant.AccountId).then(account => {
    accountId.value = account
})
get(Constant.AccountAvatar).then(a => {
    avatar.value = a
})

watch(withAccountId, async (id, _) => {
    msgArray.splice(0, msgArray.length)
    let arr = msgChannelMap.get(await msgChannelMapKey(id))
    if (arr === undefined) {
        return
    }
    for (let i = 0; i < arr.length; i ++) {
        msgArray.push(arr[i])
    }
    msgArray.reverse()
    httpClient.get('/friend/info/' + String(accountId.value) + '/' + String(id), {}, true).then(async res => {
        if (res.ok) {
            // @ts-ignore
            withRemark.value = res.data.remark
            // @ts-ignore
            withAvatar.value = BASE_URL + res.data.avatar
        }
    })
})

watch(msgChannelMap, async (map, _) => {
    msgArray.splice(0, msgArray.length)
    let arr = msgChannelMap.get(await msgChannelMapKey(withAccountId.value))
    if (arr === undefined) {
        return
    }
    for (let i = 0; i < arr.length; i ++) {
        msgArray.push(arr[i])
    }
    msgArray.reverse()
})

const logout = async () => {
    await tryClosePreviousNet()
    await router.push('/sign')
}

const home = () => {
    withAccountId.value = 0
    router.push("/home")
}

const friends = () => {
    withAccountId.value = 0
    router.push("/friends")
}

const send = async () => {
    if (text.value.endsWith('\n')) {
        text.value = text.value.substring(0, text.value.length - 1)
    }
    if (text.value === '') {
        return
    }
    const accountId = await get('AccountId')
    const head = new Head(new TextEncoder().encode(text.value).length, Type.Text, Number(accountId), Number(withAccountId.value), timestamp(), 0, 0);
    sendMsgChannel.push(new Msg(head, text.value))
    console.log('text: ' + text)
    text.value = ''
}
</script>

<template>
    <div class="home">
        <div class="layout">
            <div class="up">
                <input class="search">
                <img class="more" src="../../assets/add.png" @click="addFunc"/>
                <img class="chat" src="../../assets/chats.png" @click="home">
                <img class="contacts" src="../../assets/contacts.png" @click="friends">
                <img class="info" :src="avatar" @click="logout">
            </div>
            <div class="user-list">
                <div v-for="item in userMsgList">
                    <Suspense>
                        <UserMsgItem :with-account-id="item.key" :timestamp="item.value"></UserMsgItem>
                    </Suspense>
                </div>
                <div class="na"></div>
            </div>
            <div class="chat-area">
                <div class="user-info"></div>
                <div class="chat-item-list">
                    <div class="reverse">
                        <div v-for="msg in msgArray">
                            <ChatItem :avatar="withAvatar" :remark="withRemark" :type="msg.head.typ.valueOf()" :sender="msg.head.sender" :receiver="msg.head.receiver" :timestamp="msg.head.timestamp" :seq-num="msg.head.seq_num" :version="msg.head.version" :payload="msg.payload"></ChatItem>
                        </div>
                    </div>
                    <div class="bottom"></div>
                </div>
                <Suspense>
                    <div class="input-area">
                        <textarea class="input" @keyup.enter="send" v-model="text"></textarea>
                    </div>
                </Suspense>
            </div>
        </div>
    </div>
</template>

<style scoped>
.home {
    width: 100%;
    height: 100%;
}

.layout {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "up up"
        "user-list chat-area";
    grid-template-rows: 60px calc(100% - 60px);
    grid-template-columns: 240px 1fr;
}

.up {
    grid-area: up;
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas: "search more na1 chat friends na2 info";
    grid-template-columns: 180px 60px 80px 80px 80px 1fr 80px;
    background-color: #e7e8e8;
}

.search {
    display: inline-block;
    box-sizing: border-box;
    font-size: 1.0rem;
    grid-area: search;
    width: calc(100% - 24px);
    height: calc(100% - 32px);
    border: none;
    border-radius: 20px;
    padding: 0 0 0 8px;
    margin: 16px 12px 16px 12px;
    background-color: white;
}

.search:hover {
    outline: none;
}

.more {
    grid-area: more;
    width: calc(100% - 32px);
    height: calc(100% - 32px);
    margin: 16px 16px 16px 16px;
    border-radius: 50%;
}

.chat {
    grid-area: chat;
    width: calc(60px - 32px);
    height: calc(60px - 32px);
    margin: 16px 16px 16px 16px;
}

.contacts {
    grid-area: friends;
    width: calc(60px - 32px);
    height: calc(60px - 32px);
    margin: 16px 16px 16px 16px;
    border-radius: calc(50%);
}

.info {
    grid-area: info;
    width: calc(60px - 32px);
    height: calc(60px - 32px);
    margin: 16px 16px 16px 16px;
    border-radius: 50%;
}

.user-list {
    grid-area: user-list;
    overflow-y: scroll;
    background-color: white;
    box-sizing: border-box;
    border-right: 1px solid gainsboro;
}

.na {
    width: 100%;
}

::-webkit-scrollbar {
    display: none;
}

.chat-area {
    width: 100%;
    height: 100%;
    grid-area: chat-area;
    background-color: white;
    display: grid;
    grid-template-areas:
        "user-info"
        "chat-item-list"
        "input-area";
    grid-template-rows: 40px calc(100% - 220px) 180px;
}

.user-info {
    grid-area: user-info;
    height: 40px;
    width: 100px;
}

.chat-item-list {
    grid-area: chat-item-list;
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas: "reverse" "bottom";
    grid-template-rows: 1fr 0;
    grid-template-columns: 1fr;
}

.reverse {
    max-height: 100%;
    overflow-y: scroll;
    grid-area: reverse;
    display: flex;
    flex-direction: column-reverse;
}

.bottom {
    grid-area: bottom;
}

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
</style>
