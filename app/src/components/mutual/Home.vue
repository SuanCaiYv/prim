<script setup lang="ts">
import {useRouter} from "vue-router";
import {ref} from "vue";
import {checkNull} from "../../util/base";
import {httpClient} from "../../api/frontend/http";
import alertFunc from "../alert/alert";
import {set} from "idb-keyval";
import {startNet} from "../../function/net";
import {Constant} from "../../system/constant";

const router = useRouter()
let accountId = ref<number>()
let credential = ref<string>("")
let warnAccountId = ref<boolean>(false)
let warnCredential = ref<boolean>(false)
let infoAccountId = ref<boolean>(false)
let infoCredential = ref<boolean>(false)

const login = async () => {
    if (checkNull(accountId.value)) {
        warnAccountId.value = true
        return
    }
    if (checkNull(credential.value)) {
        warnCredential.value = true
        return
    }
    const resp = await httpClient.put("/user", {}, {
        account_id: accountId.value,
        credential: credential.value
    }, false);
    if (!resp.ok) {
        console.log(resp.errMsg)
        alertFunc(resp.errMsg, function () {
            router.push('/sign')
        })
    } else {
        await set(Constant.Authed, true)
        await set(Constant.Token, String(resp.data))
        await set(Constant.AccountId, Number(accountId.value))
        await startNet();
        await router.push('/home')
    }
}

const sign = async () => {
    if (checkNull(accountId.value)) {
        warnAccountId.value = true
        return
    }
    if (checkNull(credential.value)) {
        warnCredential.value = true
        return
    }
    const resp = await httpClient.post('/user', {}, {
        account_id: accountId.value,
        credential: credential.value
    }, false);
    if (!resp.ok) {
        alertFunc(resp.errMsg, function () {
        })
    } else {
        infoAccountId.value = true
        infoCredential.value = true
        alertFunc('done', function () {
        })
    }
}
</script>

<template>
    <div class="home">
        <div class="user-id input">
            <div class="prefix">账号</div>
            <input type="text" class="input-box" :class="{warn: warnAccountId, info: infoAccountId}"
                   v-model="accountId">
        </div>
        <div class="password input">
            <div class="prefix">密码</div>
            <input type="password" class="input-box" :class="{warn: warnCredential, info: infoCredential}"
                   v-model="credential">
        </div>
        <div class="login button">
            <button class="button-box" @click="login">登录</button>
        </div>
        <div class="sign button">
            <button class="button-box" @click="sign">注册</button>
        </div>
    </div>
</template>

<style scoped>
.home {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-areas:
        "na3 na1 na1 na4"
        "na3 user-id user-id na4"
        "na3 password password na4"
        "na3 login sign na4"
        "na3 na2 na2 na4";
    grid-template-columns: 1fr 180px 180px 1fr;
    grid-template-rows: 1fr 60px 60px 80px 2fr;
}

.input {
    border: none;
    box-sizing: border-box;
    border-radius: 16px;
    font-size: 1.4rem;
    font-weight: bolder;
    line-height: 40px;
    text-align: left;
}

.user-id {
    grid-area: user-id;
}

.password {
    grid-area: password;
}

.button {
    border: none;
    box-sizing: border-box;
    border-radius: 16px;
    font-size: 1.4rem;
    font-weight: bolder;
    line-height: 40px;
    text-align: center;
}

.login {
    grid-area: login;
}

.sign {
    grid-area: sign;
}

.prefix {
    display: inline-block;
    height: 60px;
    width: 60px;
    box-sizing: border-box;
    text-align: left;
    justify-content: center;
    line-height: 60px;
}

.input-box {
    height: calc(100% - 8px);
    display: inline-block;
    width: calc(100% - 60px);
    padding: 0 0 0 8px;
    margin: 4px 0 4px 0;
    border: none;
    font-size: 1.4rem;
    box-sizing: border-box;
    border-radius: 16px;
    vertical-align: top;
    background-color: #e7e8e8;
    color: black;
}

.input-box:focus {
    outline: none;
}

.button-box {
    height: calc(100% - 32px);
    display: inline-block;
    width: calc(100% - 60px);
    padding: 0 16px 0 16px;
    margin: 16px 0 16px 0;
    border: 0;
    box-sizing: border-box;
    border-radius: 16px;
    vertical-align: top;
    background-color: white;
    color: black;
}

.button-box:hover {
    background-color: #f7f8f8;
}

.button-box:active {
    background-color: gainsboro;
}

.button-box:focus {
    outline: none;
}

.warn {
    background-color: red;
    opacity: 20%;
}

.info {
    background-color: green;
    opacity: 20%;
}
</style>
