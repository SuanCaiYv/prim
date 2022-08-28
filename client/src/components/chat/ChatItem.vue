<script setup lang="ts">
import {defineProps, ref} from "vue";
import {Type} from "../../api/backend/entity";
import {get} from "_idb-keyval@6.2.0@idb-keyval";
import {httpClient} from "../../api/frontend";

const props = defineProps({
    avatar: String,
    remark: String,
    type: Number,
    sender: Number,
    receiver: Number,
    timestamp: Number,
    seqNum: Number,
    version: Number,
    payload: String,
})
let accountId = ref<number>(0)
let msgStr = ref<string>('')
let isSender = ref<boolean>(false)
let isOperation = ref<boolean>(false)
let comment = ref<string>('')
get('AccountId').then(account => {
    accountId.value = account
    isSender.value = account !== props.sender
})
switch (props.type) {
    case Type.Text || Type.Meme || Type.File || Type.Image || Type.Audio || Type.Video:
        // @ts-ignore
        msgStr.value = props.payload
        break;
    case Type.FriendRelationship:
        // @ts-ignore
        if (props.payload.startsWith('ADD')) {
            isOperation.value = true
            // @ts-ignore
            comment.value = props.payload.split('_')[1]
        }
}
const reject = () => {}
const ok = async () => {
    const accountId = await get('AccountId')
    httpClient.post('/friend', {}, {
        account_id: accountId,
        // @ts-ignore
        friend_account_id: props.sender,
        remark: props.remark
    }, true).then(resp => {
        if (resp.ok) {
            // console.log('add friend success')
        }
    })
}
</script>

<template>
    <div class="chat-item">
        <div v-if="isSender" class="sender">
            <img class="avatar" :src="props.avatar">
            <div class="msg">
                <div v-if="isOperation" class="inner-msg" style="float: left; width: fit-content; background-color: #D5C4A6;">
                    <span style="font-weight: bolder">{{props.sender}} 请求添加好友</span>
                    <br>
                    <span style="float: left">备注: {{comment}}</span>
                    <br>
                    <div class="button reject" @click="reject">拒绝</div>
                    <div class="button confirm" @click="ok">好</div>
                </div>
                <div v-else class="inner-msg" style="float: left; width: fit-content; background-color: gainsboro;">{{msgStr}}</div>
            </div>
            <div class="na"></div>
        </div>
        <div v-else class="receiver">
            <div class="na"></div>
            <div class="msg">
                <div v-if="isOperation" class="inner-msg" style="float: right; width: fit-content; background-color: #8C9B7C;">
                    用户: {{props.sender}} 请求添加好友
                    <br>
                    备注: {{comment}}
                    <br>
                    <div class="button reject" @click="reject">拒绝</div>
                    <div class="button confirm" @click="ok">好</div>
                </div>
                <div v-else class="inner-msg" style="float: right; width: fit-content; background-color: #d8e9dd;">{{msgStr}}</div>
            </div>
            <img class="avatar" :src="props.avatar">
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

.button {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border-radius: 12px;
    border: none;
    text-align: center;
    line-height: 30px;
    font-size: 1rem;
}

.reject {
    grid-area: reject;
    font-weight: bolder;
    width: 100%;
    height: 100%;
    background-color: #8ABCD1;
}

.confirm {
    grid-area: confirm;
    width: 100%;
    height: 100%;
    background-color: #2C9678;
}

.button:hover {
    cursor: pointer;
}
</style>
