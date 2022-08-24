<script setup lang="ts">
import {defineProps} from "vue";

const props = defineProps({
    avatar: String,
    remark: String,
    msg: String,
    type: Number,
    timestamp: Number,
    seqNum: Number,
    isSender: Boolean,
})
</script>

<template>
    <div class="chat-item">
        <div v-if="isSender" class="sender">
            <img class="avatar" src="src/assets/default-avatar.jpg">
            <div class="msg">
                <div class="inner-msg" style="float: left; width: fit-content; background-color: gainsboro;">{{props.msg}}</div>
            </div>
            <div class="na"></div>
        </div>
        <div v-else class="receiver">
            <div class="na"></div>
            <div class="msg">
                <div class="inner-msg" style="float: right; width: fit-content; background-color: #d8e9dd;">{{props.msg}}</div>
            </div>
            <img class="avatar" src="src/assets/default-avatar-2.jpg">
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
        "avatar1 msg na1";
    grid-template-rows: 60px 1fr;
    grid-template-columns: 60px 300px 1fr;
}

.receiver {
    width: 100%;
    min-height: 60px;
    display: grid;
    grid-template-areas:
        "na msg avatar"
        "na1 msg avatar1";
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
}

.inner-msg {
    min-height: 44px;
    box-sizing: border-box;
    margin: 8px 0 8px 0;
    font-size: 1rem;
    color: black;
    border: 0;
    line-height: 28px;
    border-radius: 12px;
    display: inline-block;
    padding: 8px 8px 8px 8px;
}

.na {
    grid-area: na;
}
</style>
