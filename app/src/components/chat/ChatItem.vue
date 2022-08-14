<script setup lang="ts">
import {defineProps} from "vue";

const props = defineProps({
    avatar: String,
    msg: String,
    isSender: Boolean,
})
</script>

<template>
    <div class="chat-item">
        <div v-if="isSender" class="sender">
            <img class="avatar" src="src/assets/default-avatar.jpg">
            <div class="na"></div>
            <div class="msg" style="text-align: left; background-color: gainsboro; padding: 8px 8px 8px 8px">
                我是一号
            </div>
        </div>
        <div v-else class="receiver">
            <img class="avatar" src="src/assets/default-avatar-2.jpg">
            <div class="na"></div>
            <div class="msg" style="text-align: right; background-color: #d8e9dd; padding: 8px 8px 8px 8px">
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
                我是二号我是二号我是二号我是二号我是二号我是二号我是二号我是二号
            </div>
        </div>
    </div>
</template>

<style scoped>
.chat-item {
    min-height: 60px;
    width: 100%;
}

.sender {
    width: 100%;
    min-height: 60px;
    display: grid;
    grid-template-areas:
        "avatar msg na"
        "avatar1 msg na";
    grid-template-rows: 60px 1fr;
    grid-template-columns: 60px 300px 1fr;
}

.receiver {
    width: 100%;
    min-height: 60px;
    display: grid;
    grid-template-areas:
        "na msg avatar"
        "na msg avatar1";
    grid-template-rows: 60px 1fr;
    grid-template-columns: 1fr 300px 60px;
}

.avatar {
    grid-area: avatar;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 10px 10px 10px 10px;
    border-radius: 50%;
}

.msg {
    grid-area: msg;
    width: fit-content;
    min-height: 44px;
    box-sizing: border-box;
    margin: 8px 0 8px 0;
    font-size: 1rem;
    color: black;
    border: 0;
    line-height: 28px;
    border-radius: 12px;
}

.na {
    grid-area: na;
}
</style>
