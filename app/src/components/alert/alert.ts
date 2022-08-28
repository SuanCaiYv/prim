import {createApp} from 'vue'
import AlertComponent from "./Alert.vue"

const alertFunc = function (alertMsg: string, ...afterDoneCallback: Function[]) {
    if (afterDoneCallback.length === 0) {
        afterDoneCallback.push(() => {});
    }
    let divElement = document.createElement("div");
    const instance = createApp(AlertComponent, {
        divNode: divElement,
        msg: alertMsg,
        afterDone: afterDoneCallback[0],
    })
    instance.mount(divElement)
    // @ts-ignore
    document.getElementById("app").appendChild(divElement)
}

export default alertFunc;