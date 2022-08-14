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
            <div class="avatar">
                <img :src="avatar">
            </div>
            <div class="na"></div>
            <div class="msg">
                {{msg}}
            </div>
        </div>
        <div v-else class="receiver">
            <div class="avatar">
                <img :src="avatar">
            </div>
            <div class="na"></div>
            <div class="msg">
                {{msg}}
            </div>
        </div>
    </div>
</template>

<style scoped>
.chat-item {
    height: 60px;
    width: 100%;
}

.sender {
    width: 100%;
    display: grid;
    grid-template-areas:
        "avatar na msg";
    grid-template-rows: 60px;
    grid-template-columns: 60px 300px 1fr;
    background-color: #535bf2;
}

.receiver {
    width: 100%;
    display: grid;
    grid-template-areas:
        "msg avatar";
    grid-template-rows: 60px;
    grid-template-columns: 1fr 300px 60px;
    background-color: #747bff;
}

.avatar {
    grid-area: avatar;
    width: 100%;
    height: 100%;
    background-color: #ffc107;
}

.msg {
    grid-area: msg;
    width: 100%;
    height: 100%;
    background-color: antiquewhite;
}

.na {
    grid-area: na;
}
</style>
