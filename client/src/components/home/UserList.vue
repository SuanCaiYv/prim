<script setup lang="ts">
import Item from './userList/UserListItem.vue'
import {chatList} from "../../system/net";
import {ref} from "vue";
import {get} from "idb-keyval";
import {watchEffect} from "_vue@3.2.37@vue";

let currentId = ref<number>(0)

get('AccountId').then(accountId => {
    currentId.value = accountId
})
</script>

<template>
    <div class="user-list">
        <div v-for="item in chatList">
            <Item :user-account-id="Number(item.keys().next().value)"></Item>
        </div>
        <div class="na"></div>
    </div>
</template>

<style scoped>
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
</style>
