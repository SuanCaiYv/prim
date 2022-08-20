<script setup lang="ts">
import {moreAlert, addFriend, createGroup} from './alert'
import {useRouter} from "vue-router";

const router = useRouter();

const home = () => {
    router.push("/home")
}

const friends = () => {
    router.push("/friends")
}

const f1 = () => {
    addFriend(function () {
        console.log('addFriend')
    })
}

const f2 = () => {
    console.log('f2')
}
</script>

<template>
    <div class="up">
        <input class="search">
        <img class="more" src="src/assets/tianjia-01.svg" @click="moreAlert(f1, f2)"/>
        <img class="chat" src="src/assets/chat-3.svg" @click="home">
        <img class="friends" src="src/assets/md-contacts.svg" @click="friends">
    </div>
</template>

<style scoped>
.up {
    grid-area: up;
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas: "search more na chat friends";
    grid-template-columns: 180px 60px 80px 80px 1fr;
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
    background-color: white;
}

.chat {
    grid-area: chat;
    width: calc(60px - 32px);
    height: calc(60px - 32px);
    margin: 16px 16px 16px 16px;
}

.friends {
    grid-area: friends;
    width: calc(60px - 32px);
    height: calc(60px - 32px);
    margin: 16px 16px 16px 16px;
    border-radius: calc(50%);
}
</style>
