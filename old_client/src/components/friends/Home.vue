<script setup lang="ts">
import {useRouter} from "vue-router";
import {addFunc} from "../common/jetcomponents";
import {reactive} from "vue";
import {BASE_URL, httpClient} from "../../api/frontend/http";
import FriendListItem from './FriendListItem.vue'
import {tryClosePreviousNet} from "../../function/net";
import {AccountAvatar, AccountId} from "../../function/types";

const router = useRouter()
let friendList = reactive<Array<any>>([])

const logout = async () => {
    await tryClosePreviousNet()
    await router.push('/sign')
}

const home = () => {
    router.push("/home")
}

const friends = () => {
    router.push("/friends")
}
httpClient.get('/friend/list/' + String(AccountId.value), {}, true).then(async resp => {
    if (resp.ok) {
        let list = resp.data as Array<any>
        for (let i = 0; i < list.length; i++) {
            friendList.push({
                accountId: list[i].account_id,
                // @ts-ignore
                remark: list[i].remark,
                // @ts-ignore
                avatar: BASE_URL + list[i].avatar
            })
        }
    }
})
</script>

<template>
    <div class="friends">
        <div class="layout">
            <div class="up">
                <input class="search">
                <img class="more" src="../../assets/add.png" @click="addFunc"/>
                <img class="chat" src="../../assets/chats.png" @click="home">
                <img class="contacts" src="../../assets/contacts.png" @click="friends">
                <img class="info" :src="AccountAvatar" @click="logout">
            </div>
            <div class="friend-list">
                <div v-for="friend in friendList">
                    <FriendListItem :avatar="friend.avatar" :remark="friend.remark"
                                    :account-id="friend.accountId"></FriendListItem>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.friends {
    width: 100%;
    height: 100%;
}

.layout {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "up up"
        "friend-list na";
    grid-template-rows: 60px 1fr;
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

.friend-list {
    grid-area: friend-list;
    overflow-y: scroll;
    background-color: white;
    box-sizing: border-box;
    border-right: 1px solid gainsboro;
}

::-webkit-scrollbar {
    display: none;
}
</style>
