import { HttpClient } from "../../net/http";
import { KVDB } from "../database"

export class UserInfo {
    static avatarNickname = async (userId: bigint): Promise<[string, string]> => {
        let avatar = await KVDB.get(`avatar-${userId}`);
        let nickname = await KVDB.get(`nickname-${userId}`);
        if (avatar === undefined || nickname === undefined) {
            let resp = await HttpClient.get("/user/s-info-n", {
                user_id: userId,
            }, false);
            if (!resp.ok) {
                console.log(resp.errMsg);
                return ["", ""];
            }
            let data = resp.data;
            avatar = data.avatar;
            nickname = data.nickname;
            await KVDB.set(`avatar-${userId}`, avatar);
            await KVDB.set(`nickname-${userId}`, nickname);
        }
        return [avatar, nickname];
    }

    static avatarRemark = async (userId: bigint, peerId: bigint): Promise<[string, string]> => {
        let avatar = await KVDB.get(`avatar-${peerId}`);
        let remark = await KVDB.get(`remark-${userId}-${peerId}`);
        if (avatar === undefined || remark === undefined) {
            let resp = await HttpClient.get("/user/s-info-r", {
                peer_id: peerId,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                return ["", ""];
            }
            let data = resp.data;
            avatar = data.avatar;
            remark = data.remark;
            await KVDB.set(`avatar-${userId}`, avatar);
            await KVDB.set(`remark-${userId}-${peerId}`, remark);
        }
        return [avatar, remark];
    }
}