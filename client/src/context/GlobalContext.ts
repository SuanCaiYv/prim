import React from 'react';
import { Msg } from "../entity/msg"

class Context {
    userMsgList: Array<Msg>
    msgMap: Map<string, Array<Msg>>
    contactList: Array<any>
    userId: bigint
    userAvatar: string
    userNickname: string
    setUserMsgList: (msgList: Array<Msg>, ...cb: (() => void)[]) => void
    setMsgMap: (msgMap: Map<string, Array<Msg>>, ...cb: (() => void)[]) => void
    setContactList: (contactList: Array<any>, ...cb: (() => void)[]) => void
    setUserId: (userId: bigint, ...cb: (() => void)[]) => void
    setUserAvatar: (userAvatar: string, ...cb: (() => void)[]) => void
    setUserNickname: (userNickname: string, ...cb: (() => void)[]) => void
    constructor() {
        this.userMsgList = []
        this.msgMap = new Map()
        this.contactList = []
        this.userId = BigInt(0)
        this.userAvatar = ""
        this.userNickname = ""
        this.setUserMsgList = (msgList: Array<Msg>) => { }
        this.setMsgMap = (msgMap: Map<string, Array<Msg>>) => { }
        this.setContactList = (contactList: Array<any>) => { }
        this.setUserId = (userId: bigint) => { }
        this.setUserAvatar = (userAvatar: string) => { }
        this.setUserNickname = (userNickname: string) => { }
    }
}

const GlobalContext = React.createContext(new Context())

export { GlobalContext, Context }