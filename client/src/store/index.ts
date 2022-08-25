import {createStore, Store} from "vuex";
import {Client} from "../api/backend/api";

const store: Store<any> = createStore({
    state() {
        return {
            connected: Boolean,
            netApi: Client,
        }
    },
    mutations: {
        updateConnected(state, connected) {
            state.connected = connected
        },
        updateNetApi(state, netApi) {
            state.netApi = netApi
        }
    },
    getters: {
        connected: (state) => {
            return state.connected
        },
        netApi: (state) => {
            return state.netApi
        }
    }
})

export default store