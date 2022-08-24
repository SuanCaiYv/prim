<script setup lang="ts">
import {provide, reactive} from "_vue@3.2.37@vue";

const addFriendList = reactive<Array<any>>([])
provide('addFriendList', addFriendList)
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
