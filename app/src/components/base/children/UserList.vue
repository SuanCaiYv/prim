<script setup lang="ts">
import Item from './userList/UserListItem.vue'
</script>

<template>
    <div class="user-list">
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
        <Item></Item>
    </div>
</template>

<style scoped>
.user-list {
    grid-area: user-list;
    overflow-y: scroll;
}

::-webkit-scrollbar {
    display: none;
}
</style>
