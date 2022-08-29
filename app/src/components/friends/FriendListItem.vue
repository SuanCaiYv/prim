<script setup lang="ts">
import {defineProps} from "vue"
import {useRouter} from "vue-router";
import {useStore} from "vuex";
import {get} from "idb-keyval";
import {userMsgSet, withAccountId} from "../../function/types";
import {Constant} from "../../system/constant";
import {timestamp} from "../../util/base";

const router = useRouter();

const props = defineProps({
    avatar: String,
    remark: String,
    accountId: Number
});

const chat = async () => {
    if (props.accountId !== undefined) {
        let map = userMsgSet.get(Number(await get(Constant.AccountId)));
        if (map === undefined) {
            map = new Map<number, number>();
            userMsgSet.set(Number(await get(Constant.AccountId)), map);
        }
        // @ts-ignore
        userMsgSet.get(Number(await get(Constant.AccountId))).set(props.accountId, timestamp())
        withAccountId.value = 0
        withAccountId.value = props.accountId
    }
    await router.push("/home")
}

</script>

<template>
    <div class="user-list-item">
        <img class="avatar" :src="props.avatar" @dblclick="chat">
        <div class="remark">{{ props.remark }}</div>
        <div class="id">{{ props.accountId }}</div>
    </div>
</template>

<style scoped>
.user-list-item {
    height: 60px;
    width: 100%;
    display: grid;
    grid-template-areas:
            "avatar remark"
            "avatar id";
    grid-template-rows: 30px 30px;
    grid-template-columns: 60px 1fr;
}

.avatar {
    grid-area: avatar;
    width: calc(100% - 12px);
    height: calc(100% - 12px);
    margin: 6px 6px 6px 6px;
    border-radius: 50%;
}

.avatar:hover {
    cursor: pointer;
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

.id {
    grid-area: id;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: 0 0 0 8px;
    font-size: 1.0rem;
    overflow-x: hidden;
    text-align: left;
    line-height: 30px;
    color: black;
}
</style>
