<script setup lang="ts">
import Item from './userList/UserListItem.vue'
import {inject, ref} from "vue";
import {get} from "idb-keyval";

let currentId = ref<number>(0)
get('AccountId').then(accountId => {
    currentId.value = accountId
})

let chatList = inject('chatList')
</script>

<template>
    <div class="user-list">
        <div v-for="item in chatList">
            <Item :user-account-id="item"></Item>
        </div>
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

::-webkit-scrollbar {
    display: none;
}
</style>
