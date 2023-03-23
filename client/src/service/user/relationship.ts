import { ContactItemData } from "../../entity/inner";
import { HttpClient } from "../../net/http";
import { KVDB } from "../database";

export default class Relationship {
    static contactList = async (): Promise<Array<ContactItemData>> => {
        let userId = await KVDB.get("user-id");
        let obj = await KVDB.get(`contact-list-${userId}`);
        if (obj === undefined) {
            let list = await HttpClient.get("/relationship/friend", {}, true);
            if (!list.ok) {
                console.log(list.errMsg);
                return [];
            }
        }
    }
}