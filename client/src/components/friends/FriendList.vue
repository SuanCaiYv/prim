<script setup lang="ts">
import Item from "./FriendListItem.vue";
import {reactive, ref} from "_vue@3.2.37@vue";
import {httpClient} from "../../api/frontend";
import {get} from "_idb-keyval@6.2.0@idb-keyval";
import {inject} from "vue";

let accountId = ref<number>(0)
get('AccountId').then(account => {
    accountId.value = account
    console.log(account)
    httpClient.get('/friend/list/' + String(accountId.value), {}, true).then(async resp => {
        if (resp.ok) {
            let list = resp.data as Array<number>
            for (let i = 0; i < list.length; i++) {
                httpClient.get('/friend/info/' + String(accountId.value) + String(list[i]), {}, true).then(async res => {
                    if (res.ok) {
                        friendList.push({
                            id: list[i],
                            // @ts-ignore
                            remark: res.data.remark,
                            // @ts-ignore
                            avatar: res.data.avatar
                        })
                    }
                })
            }
        }
    })
})
const friendList = reactive<Array<any>>([])
let addFriendList = inject('addFriendList') as Array<any>
</script>

<template>
    <div class="friend-list">
        <div v-for="friend in friendList">
            <Item :remark="friend.remark" :user-id="friend.userId"></Item>
        </div>
        <div v-for="friend in addFriendList">
            <Item :remark="friend.remark" :user-id="friend.userId" :add-friend="true"></Item>
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
