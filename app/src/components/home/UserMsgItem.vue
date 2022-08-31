<script async setup lang="ts">
import {defineProps, ref} from "vue";
import {get} from "idb-keyval";
import {Constant} from "../../system/constant";
import {BASE_URL, httpClient} from "../../api/frontend/http";
import {msgChannelMap, withAccountId} from "../../function/types";
import {msgChannelMapKey} from "../../function/net";

let accountId = ref<number>(0)
let withAvatar = ref<string>('')
let remark = ref<string>('')
let time = ref<string>('')
let shotMsg = ref<string>('')

const props = defineProps({
    withAccountId: Number,
    timestamp: Number,
})

httpClient.get('/user/info/' + String(props.withAccountId), {}, false).then(async res => {
    if (res.ok) {
        // @ts-ignore
        remark.value = res.data.remark
        // @ts-ignore
        withAvatar.value = BASE_URL + res.data.avatar
    } else {
        console.log('error: ', res.errMsg)
    }
})

const clickFunc = () => {
    // @ts-ignore
    withAccountId.value = props.withAccountId
}

const loadShotMsg = async() => {
    const key = await msgChannelMapKey(Number(props.withAccountId))
    const date = new Date(Number(props.timestamp));
    time.value = date.toLocaleDateString() + ' ' + date.toLocaleTimeString()
    // @ts-ignore
    if (msgChannelMap.get(key) !== undefined && msgChannelMap.get(key).length !== 0) {
        // @ts-ignore
        shotMsg.value = msgChannelMap.get(key)[msgChannelMap.get(key).length-1].payload
        if (shotMsg.value.startsWith("ADD_")) {
            shotMsg.value = shotMsg.value.split('_')[1]
        } else if (shotMsg.value.startsWith("COMPLETE")) {
            shotMsg.value = '已添加好友'
        }
    } else {
        shotMsg.value = ''
    }
}
await loadShotMsg()
</script>

<template>
    <div class="user-list-item" @click="clickFunc">
        <img class="avatar" :src="withAvatar">
        <div class="remark">{{ remark }}</div>
        <div class="short-msg">{{ shotMsg }}</div>
        <div class="time">{{ time }}</div>
        <div class="count">
            <div class="number"></div>
        </div>
    </div>
</template>

<style scoped>
.user-list-item {
    height: 60px;
    width: 100%;
    display: grid;
    grid-template-areas:
            "avatar remark time"
            "avatar short-msg count";
    grid-template-rows: 30px 30px;
    grid-template-columns: 60px 1fr 100px;
}

.avatar {
    grid-area: avatar;
    width: calc(100% - 12px);
    height: calc(100% - 12px);
    margin: 6px 6px 6px 6px;
    border-radius: 50%;
}

.remark {
    grid-area: remark;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 1.0rem;
    font-weight: bolder;
    text-align: left;
    line-height: 30px;
    color: black;
}

.short-msg {
    grid-area: short-msg;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 0.8rem;
    overflow-x: hidden;
    text-align: left;
    line-height: 30px;
    color: black;
}

.time {
    grid-area: time;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 4px;
    font-size: 0.6rem;
    text-align: center;
    line-height: 30px;
    color: black;
}

.count {
    grid-area: count;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 64px;
    font-size: 0.8rem;
    text-align: right;
    line-height: 30px;
    background-color: white;
}

.number {
    height: 14px;
    width: 14px;
    margin: 8px 0 0 16px;
    border-radius: 100%;
    background-color: red;
    font-size: 0.8rem;
    font-weight: bolder;
    text-align: right;
    line-height: 14px;
    color: white;
}
</style>
