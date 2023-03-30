import { ContactItemData } from "../../entity/inner";
import { HttpClient } from "../../net/http";
import { KVDB } from "../database";
import { UserInfo } from "./userInfo";

export default class Relationship {
    static contactList = async (): Promise<Array<ContactItemData>> => {
        // let userId = await KVDB.get("user-id");
        // let list;
        // let obj = await KVDB.get(`contact-list-${userId}`);
        // if (obj === undefined) {
        //     let resp = await HttpClient.get("/relationship/friend", {
        //         offset: 0,
        //         number: 100,
        //     }, true);
        //     if (!resp.ok) {
        //         console.log(resp.errMsg);
        //         return [];
        //     }
        //     let arr = resp.data as Array<any>;
        //     list = [];
        //     for (let i = 0; i < arr.length; ++ i) {
        //         let peerId = BigInt(arr[i].peer_id);
        //         let remark = arr[i].remark as string;
        //         let [avatar, nickname] = await UserInfo.avatarNickname(peerId);
        //         list.push(new ContactItemData(peerId, avatar, remark, nickname));
        //     }
        //     await KVDB.set(`contact-list-${userId}`, list);
        // } else {
        //     list = obj as Array<ContactItemData>;
        // }
        let resp = await HttpClient.get("/relationship/friend", {
            offset: 0,
            number: 100,
        }, true);
        if (!resp.ok) {
            console.log(resp.errMsg);
            return [];
        }
        let arr = resp.data as Array<any>;
        let list = [];
        for (let i = 0; i < arr.length; ++ i) {
            let peerId = BigInt(arr[i].peer_id);
            let remark = arr[i].remark as string;
            let [avatar, nickname] = await UserInfo.avatarNickname(peerId);
            list.push(new ContactItemData(peerId, avatar, remark, nickname));
        }
        return list;
    }
}