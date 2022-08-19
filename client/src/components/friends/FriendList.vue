<script setup lang="ts">
import Item from "./FriendListItem.vue";
import {reactive} from "_vue@3.2.37@vue";

const friendList = reactive<Array<any>>([])
friendList.push({
    avatar: "",
    remark: "马",
    userId: 1,
})
friendList.push({
    avatar: "",
    remark: "李",
    userId: 2,
})
</script>

<template>
    <div class="friend-list">
        <div v-for="friend in friendList">
            <Item :remark="friend.remark" :user-id="friend.userId"></Item>
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
