<template>
    <div class="alert">
        <p class="alert-message">{{ msg }}</p>
        <button class="alert-button" @click="close">确定</button>
    </div>
    <div class="alert-mask" @click.self="close"></div>
</template>

<script setup lang="ts">
import {ref} from "vue"

const name = ref<String>("Alert")

const props = defineProps({
    divNode: Node,
    msg: String,
    afterDone: Function
})

const close = function () {
    // @ts-ignore
    document.getElementById("app").removeChild(props.divNode)
    // @ts-ignore
    props.afterDone()
}
</script>

<style scoped>
.alert {
    position: absolute;
    width: 320px;
    left: 50%;
    transform: translate(-50%, 0);
    top: 20%;
    background: white;
    border-radius: 20px;
    padding: 24px;
    z-index: 1001;
}

.alert-message {
    font-size: 1rem;
    font-weight: bolder;
    line-height: 22px;
    color: black;
    margin-bottom: 32px;
}

.alert-button {
    min-width: 80px;
    padding: 8px 24px;
    text-align: center;
    background: dodgerblue;
    border: 0;
    outline: 0;
    color: white;
    font-size: 1rem;
    font-weight: bolder;
    border-radius: 18px;
    cursor: pointer;
}

.alert-mask {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    background-color: black;
    opacity: 0.5;
    z-index: 1000;
}
</style>