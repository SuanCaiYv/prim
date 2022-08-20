<template>
    <div class="add-friend">
        <div class="name">账号</div>
        <input type="text" class="input">
        <div class="name">备注</div>
        <input type="text" class="input">
        <button class="button" @click="f">发送</button>
    </div>
    <div class="mask" @click.self="close"></div>
</template>

<script setup lang="ts">
import {ref} from "vue"

const name = ref<string>("AddFriend")

const props = defineProps({
    divNode: Node,
    f: Function,
})

const close = function () {
    // @ts-ignore
    document.getElementById("app").removeChild(props.divNode)
}
</script>

<style scoped>
.add-friend {
    position: absolute;
    width: 200px;
    height: 100px;
    left: 50%;
    transform: translate(-50%, 0);
    top: 200px;
    border-radius: 8px;
    padding: 0;
    z-index: 1001;
    background-color: white;
}

.name {
    display: inline-block;
    box-sizing: border-box;
    border: 0;
    color: black;
    width: 40px;
    padding: 0 0 0 8px;
    font-weight: bolder;
    line-height: 30px;
    text-align: left;
    font-size: 0.8rem;
    height: 30px;
}

.input {
    display: inline-block;
    box-sizing: border-box;
    border-radius: 8px;
    border: 0;
    width: calc(160px - 12px);
    height: calc(30px - 8px);
    margin: 4px 4px 4px 8px;
    padding: 0 0 0 8px;
    font-size: 0.8rem;
    color: black;
    font-weight: bolder;
    line-height: 22px;
    text-align: left;
    background-color: #e7e8e8;
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

.button {
    box-sizing: border-box;
    padding: 0;
    margin: 10px 0 0 0;
    border: 0;
    height: 30px;
    width: 200px;
    font-size: 0.8rem;
}

.button:hover {
    outline: none;
}

.button:active {
    outline: none;
}

.button:focus {
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