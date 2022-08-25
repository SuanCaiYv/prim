import {createApp} from 'vue'
import SearchAlert from "./SearchAlert.vue"
import AddFriend from './AddFriend.vue'
import CreateGroup from './CreateGroup.vue'

const moreAlert = function (f1: Function, f2: Function) {
    let divElement = document.createElement("div");
    const instance = createApp(SearchAlert, {
        divNode: divElement,
        addFriend: f1,
        createGroup: f2,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

const addFriend = (addFriendFunc: Function) => {
    let divElement = document.createElement("div");
    const instance = createApp(AddFriend, {
        divNode: divElement,
        addFriendFunc: addFriendFunc,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

const createGroup = (f: Function) => {
    let divElement = document.createElement("div");
    const instance = createApp(CreateGroup, {
        divNode: divElement,
        f: f,
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

export {moreAlert, addFriend, createGroup};