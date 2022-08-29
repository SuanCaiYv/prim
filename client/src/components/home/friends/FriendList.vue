<script setup lang="ts">
import Item from "./FriendListItem.vue";
import {reactive, ref} from "_vue@3.2.37@vue";
import {BASE_URL, httpClient} from "../../../api/frontend";
import {get} from "_idb-keyval@6.2.0@idb-keyval";

let accountId = ref<number>(0)
get('AccountId').then(account => {
    accountId.value = account
    httpClient.get('/friend/list/' + String(accountId.value), {}, true).then(async resp => {
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
})
const friendList = reactive<Array<any>>([])
</script>

<template>
    <div class="friend-list">
        <div v-for="friend in friendList">
            <Item :avatar="friend.avatar" :remark="friend.remark" :account-id="friend.accountId"></Item>
        </div>
    </div>
</template>

<style scoped>
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
