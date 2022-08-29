import {createStore, Store} from "vuex";

const store: Store<any> = createStore({
    state() {
        return {
            connected: false,
            netApi: null,
        }
    },
    mutations: {
        updateConnected(state, connected) {
            state.connected = connected
        },
        updateNetApi(state, netApi) {
            state.netApi = netApi
        },
    },
    getters: {
        connected: (state) => {
            return state.connected
        },
        netApi: (state) => {
            return state.netApi
        },
    }
})


export default store;