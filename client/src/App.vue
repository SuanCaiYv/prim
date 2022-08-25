<script setup lang="ts">
import {reactive, ref} from "_vue@3.2.37@vue";
import {provide} from "vue";
import {KV} from "./api/frontend/interface";

let chatSet = reactive<Set<number>>(new Set())
provide('chatSet', chatSet)
let currentChatUserAccountId = ref<number>(0)
provide('currentChatUserAccountId', currentChatUserAccountId)
let sendMsgChannel = reactive(new Array<KV<string, number>>())
provide('sendMsgChannel', sendMsgChannel)
</script>

<template>
    <router-view></router-view>
</template>

<style scoped>
#app {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-align: center;
    font-size: 20px;
    height: 100%;
    width: 100%;
}
</style>
