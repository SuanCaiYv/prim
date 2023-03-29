import { Type } from "./msg"

export class UserMsgListItemData {
    peerId: bigint
    avatar: string
    remark: string
    preview: string
    timestamp: bigint
    unreadNumber: number
    rawType: Type
    rawPayload: Array<number>
    rawExtension: Array<number>
    constructor(peerId: bigint, avatar: string, remark: string, preview: string, timestamp: bigint, unreadNumber: number, rawType: Type, rawPayload: Array<number>, rawExtension: Array<number>) {
        this.peerId = peerId
        this.avatar = avatar
        this.remark = remark
        this.preview = preview
        this.timestamp = timestamp
        this.unreadNumber = unreadNumber
        this.rawType = rawType
        this.rawPayload = rawPayload
        this.rawExtension = rawExtension
    }
}

export class ContactItemData {
    userId: bigint
    avatar: string
    remark: string
    nickname: string
    constructor(userId: bigint, avatar: string, remark: string, nickname: string) {
        this.userId = userId
        this.avatar = avatar
        this.remark = remark
        this.nickname = nickname
    }
}