<script setup lang="ts">
import {defineProps, inject, ref} from "vue";
import {BASE_URL, httpClient} from "../../../api/frontend";
import {get} from "idb-keyval";
import {msgListMap, msgListMapKey, withId} from "../../../system/net";

let accountId = ref<number>(0)
let avatar = ref<string>('')
let remark = ref<string>('')
let time = ref<string>('')
let shotMsg = ref<string>('')

get('AccountId').then(account => {
    accountId.value = account
})

const props = defineProps({
    userAccountId: Number,
})

httpClient.get('/friend/info/' + String(accountId.value) + '/' + String(props.userAccountId), {}, true).then(async res => {
    if (res.ok) {
        // @ts-ignore
        remark.value = res.data.remark
        // @ts-ignore
        avatar.value = BASE_URL + res.data.avatar
    } else {
        console.log('error: ', res.errMsg)
    }
})

const clickFunc = () => {
    console.log('clicked')
    console.log(withId.value)
    // @ts-ignore
    withId.value = props.userAccountId
    console.log(withId.value)
}

msgListMapKey(Number(props.userAccountId)).then(key => {
    let msgArray = msgListMap.get(key)
    if (msgArray === undefined || msgArray.length === 0) {
        shotMsg.value = '暂无消息'
    } else {
        shotMsg.value = msgArray[0].payload
        time.value = new Date(msgArray[0].head.timestamp).getTime().toString()
    }
})

</script>

<template>
    <div class="user-list-item" @click="clickFunc">
        <img class="avatar" :src="avatar">
        <div class="remark">{{ remark }}</div>
        <div class="short-msg">{{ shotMsg }}</div>
        <div class="time">{{ time }}</div>
        <div class="count">
            <div class="number">1</div>
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
    grid-template-columns: 60px 1fr 40px;
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
    font-size: 0.8rem;
    text-align: left;
    line-height: 30px;
    color: black;
}

.count {
    grid-area: count;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 4px;
    font-size: 0.8rem;
    text-align: left;
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
    text-align: center;
    line-height: 14px;
    color: white;
}
</style>
