<script setup lang="ts">
import Up from "../../base/children/Up.vue";
import FriendList from "./FriendList.vue";
import {ref} from "vue";
</script>

<template>
    <div class="layout">
        <Up></Up>
        <FriendList></FriendList>
    </div>
</template>

<style scoped>
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
</style>
