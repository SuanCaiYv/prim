<script setup lang="ts">
import {useRouter} from "vue-router";
import {ref} from "vue";
import {checkNull} from "../../util/base";
import {BASE_URL, httpClient} from "../../api/frontend/http";
import alertFunc from "../alert/alert";
import {set} from "idb-keyval";
import {startNet} from "../../function/net";
import {Constant} from "../../system/constant";
import {AccountAvatar, AccountId, Authed, Token} from "../../function/types";

const router = useRouter()
let accountId = ref<string>('')
let credential = ref<string>('')
let warnAccountId = ref<boolean>(false)
let warnCredential = ref<boolean>(false)
let infoAccountId = ref<boolean>(false)
let infoCredential = ref<boolean>(false)

const login = () => {
    if (checkNull(accountId.value)) {
        warnAccountId.value = true
        return
    }
    if (checkNull(credential.value)) {
        warnCredential.value = true
        return
    }
    httpClient.put("/user", {}, {
        account_id: accountId.value,
        credential: credential.value
    }, false).then(resp => {
        if (!resp.ok) {
            console.log(resp.errMsg)
            alertFunc(resp.errMsg, function () {
                router.push('/sign')
            })
        } else {
            set(Constant.Authed, true)
            set(Constant.Token, String(resp.data))
            set(Constant.AccountId, Number(accountId.value))
            Authed.value = true
            Token.value = String(resp.data)
            AccountId.value = Number(accountId.value)
            startNet()
            httpClient.get('/user/info/' + accountId.value, {}, true).then(resp => {
                if (resp.ok) {
                    // @ts-ignore
                    console.log(BASE_URL + resp.data.avatar)
                    // @ts-ignore
                    set(Constant.AccountAvatar, BASE_URL + resp.data.avatar)
                    // @ts-ignore
                    AccountAvatar.value = BASE_URL + resp.data.avatar
                } else {
                    alertFunc(resp.errMsg)
                }
            })
            router.push('/home')
        }
    });
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
            <div class="prefix">??????</div>
            <input type="text" class="input-box" :class="{warn: warnAccountId, info: infoAccountId}"
                   v-model="accountId">
        </div>
        <div class="password input">
            <div class="prefix">??????</div>
            <input type="password" class="input-box" :class="{warn: warnCredential, info: infoCredential}"
                   v-model="credential">
        </div>
        <div class="login button">
            <button class="button-box" @click="login" @keyup.enter="login">??????</button>
        </div>
        <div class="sign button">
            <button class="button-box" @click="sign">??????</button>
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
