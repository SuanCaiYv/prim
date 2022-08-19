<template>
    <div class="add-friend">
        <button class="input" @click="ff1">添加好友/群</button>
        <button class="input" @click="ff2">创建群聊</button>
    </div>
    <div class="mask" @click.self="close"></div>
</template>

<script setup lang="ts">
import {ref} from "vue"

const name = ref<string>("Alert")

const props = defineProps({
    divNode: Node,
    f1: Function,
    f2: Function,
})

const ff1 = () => {
    close()
    props.f1()
}

const ff2 = () => {
    close()
    props.f2()
}

const close = function () {
    // @ts-ignore
    document.getElementById("app").removeChild(props.divNode)
}
</script>

<style scoped>
.add-friend {
    position: absolute;
    width: 160px;
    height: 60px;
    left: 220px;
    transform: translate(0, 0);
    top: 40px;
    background: white;
    border-radius: 8px;
    padding: 0;
    z-index: 1001;
}

.input {
    display: inline-block;
    box-sizing: border-box;
    border: 0;
    width: 100%;
    font-size: 0.6rem;
    color: black;
    font-weight: bolder;
    line-height: 30px;
    padding: 0 0 0 12px;
    text-align: left;
    background-color: white;
}

.input:hover {
    outline: none;
}

.input:active {
    outline: none;
}

.input:focus {
    outline: none;
}

.mask {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    background-color: white;
    opacity: 0;
    z-index: 1000;
}
</style>