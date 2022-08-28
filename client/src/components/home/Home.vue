<script setup lang="ts">
import Layout from './Layout.vue'
import {useRouter} from "vue-router"
import {BASE_URL, httpClient} from "../../api/frontend";
import alertFunc from "../../util/alert";
import {get, set} from "idb-keyval";
import {useStore} from "vuex";
import {startNetApi} from "../../system/net";

const router = useRouter()
const store = useStore()
let accountId = get('AccountId')


get('Authed').then(async authed => {
    if (!authed) {
        await router.push('/sign')
    } else {
        await router.push('/')
    }
})

if (store.getters.netApi === undefined || store.getters.netApi === null) {
    startNetApi()
}

accountId.then(accountId => {
    httpClient.get('/user/info/' + accountId, {}, true).then(resp => {
        if (!resp.ok) {
            alertFunc(resp.errMsg, function () {
                router.push('/sign')
            })
        } else {
            // @ts-ignore
            set('AccountAvatar', BASE_URL + resp.data.avatar)
        }
    })
})
</script>

<template>
    <div class="home">
        <Layout></Layout>
    </div>
</template>

<style scoped>
.home {
    width: 100%;
    height: 100%;
}
</style>
