import {createStore, Store} from "vuex";

const store: Store<any> = createStore({
    state() {
        return {
            connected: false,
        }
    },
    mutations: {
        updateConnected(state, connected) {
            state.connected = connected
        },
    },
    getters: {
        connected: (state) => {
            return state.connected
        },
    }
})


export default store;