import Chat from "./components/chat/Chat";
import Contacts from "./components/contacts/Contacts";
import More from "./components/more/More";
import './App.css'
import { GlobalContext, UserMsgListItemData } from "./context/GlobalContext";
import { createRef, ReactNode, useState } from "react";
import { Msg, Type } from "./entity/msg";
import React from "react";
import { randomMsg } from "./mock/chat";
import Login from "./components/login/Login";
import { Client } from "./net/core";
import { KVDB, MsgDB } from "./service/database";
import { HttpClient } from "./net/http";
import { BrowserRouter, Route, Routes } from "react-router-dom";

class Props { }

class State {
    userMsgList: Array<UserMsgListItemData> = [];
    msgMap: Map<bigint, Msg[]> = new Map();
    contactList: Array<any> = [];
    userId: bigint = 1n;
    userAvatar: string = "/src/assets/avatar/default-avatar-1.png";
    userNickname: string = "prim-user";
    nodeId: number = 0;
    currentChatMsgList: Array<Msg> = [];
    currentChatPeerId: bigint = 0n;
    currentChatPeerAvatar: string = "";
    currentChatPeerRemark: string = "";
    unAckSet: Set<string> = new Set();
    contactUserId: bigint = 1n;
    savedAckMap: Map<bigint, bigint> = new Map();
    loginRedirect: () => void = () => { };
}

class App extends React.Component<Props, State> {
    netConn: Client | undefined;
    login: boolean = false;
    loginRedirect: React.RefObject<any>;
    constructor(props: any) {
        super(props);
        this.state = new State();
        this.loginRedirect = createRef();
    }

    clearState = () => {
        this.setState({
            userMsgList: [],
            msgMap: new Map(),
            contactList: [],
            userId: 0n,
            userAvatar: "",
            userNickname: "",
            nodeId: 0,
            currentChatMsgList: [],
            currentChatPeerId: 0n,
            currentChatPeerAvatar: "",
            currentChatPeerRemark: "",
            unAckSet: new Set(),
            contactUserId: 0n,
            savedAckMap: new Map(),
        });
    }

    peerId = (id1: bigint, id2: bigint) => {
        if (this.state.userId === id1) {
            return id2;
        } else {
            return id1;
        }
    }

    setLoginRedirect = (redirect: () => void) => {
        this.setState({ loginRedirect: redirect });
    }

    setUserMsgListItemUnread = (peerId: bigint, unread: boolean) => {
        let list = this.state.userMsgList;
        let newList = list.map((item) => {
            if (item.peerId === peerId) {
                item.unreadNumber = unread ? 1 : 0;
            }
            return item;
        });
        this.setState({ userMsgList: newList });
    }

    setUserMsgList = (msg: Msg) => {
        let peerId = this.peerId(msg.head.sender, msg.head.receiver);
        let text = msg.payloadText();
        let timestamp = msg.head.timestamp;
        // @todo: avatar
        let avatar = "/src/assets/avatar/default-avatar-" + peerId + ".png";
        // @todo: nickname
        let nickname = "prim-user-" + peerId;
        let number = 0;
        let list = this.state.userMsgList;
        let newList
        let v = list.find((item) => {
            return item.peerId === peerId;
        });
        // Ack will trigger resort of user msg list
        if (msg.head.type === Type.Ack) {
            if (v !== undefined) {
                number = v.unreadNumber;
                newList = [new UserMsgListItemData(peerId, avatar, nickname, v.text, timestamp, number), ...list.filter((item) => {
                    return item.peerId !== peerId;
                })]
            } else {
                newList = list;
            }
        } else {
            if (v !== undefined) {
                if (msg.head.sender === peerId) {
                    number = v.unreadNumber + 1;
                } else {
                    number = v.unreadNumber;
                }
                newList = [new UserMsgListItemData(peerId, avatar, nickname, text, timestamp, number), ...list.filter((item) => {
                    return item.peerId !== peerId;
                })]
            } else {
                if (msg.head.sender === peerId) {
                    number = 1;
                } else {
                    number = 0;
                }
                newList = [new UserMsgListItemData(peerId, avatar, nickname, text, timestamp, number), ...list];
            }
        }
        this.setState({ userMsgList: newList });
    }

    setMsgMap = (msg: Msg) => {
        let peerId = this.peerId(msg.head.sender, msg.head.receiver);
        let map = this.state.msgMap;
        let list = map.get(peerId);
        if (msg.head.type === Type.Ack) {
            // @ts-ignore
            let timestamp = BigInt(msg.payloadText())
            if (list !== undefined) {
                for (let i = list.length - 1; i >= 0; --i) {
                    if (list[i].head.sender === msg.head.sender && list[i].head.receiver === msg.head.receiver && list[i].head.timestamp === timestamp) {
                        list[i].head.timestamp = msg.head.timestamp;
                        list[i].head.seqNum = msg.head.seqNum;
                        break;
                    }
                }
                let list1 = list.filter((item) => {
                    return item.head.seqNum !== 0n;
                });
                let list2 = list.filter((item) => {
                    return item.head.seqNum === 0n;
                });
                let newList = [...list1, ...list2];
                map.set(peerId, newList);
            }
        } else {
            if (list === undefined) {
                map.set(peerId, [msg]);
            } else {
                // todo seqNum same check
                list.push(msg);
            }
            if (peerId === this.state.currentChatPeerId) {
                this.setState({ currentChatMsgList: [...this.state.currentChatMsgList, msg] });
            }
        }
    }

    setUnAckSet = (msg: Msg) => {
        if (msg.head.seqNum !== 0n) {
            return;
        }
        let key = msg.head.sender + "-" + msg.head.receiver + "-" + msg.head.timestamp;
        // @todo timeout reset
        setTimeout(() => {
            let set = this.state.unAckSet;
            set.add(key);
            this.setState({ unAckSet: set });
        }, 3000)
    }

    setAckSet = (msg: Msg) => {
        // @ts-ignore
        let timestamp = BigInt(msg.payloadText());
        let set = this.state.unAckSet;
        let key = msg.head.sender + "-" + msg.head.receiver + "-" + timestamp;
        set.delete(key);
        let newSet = new Set(set);
        this.setState({ unAckSet: newSet });
    }

    newMsg = (msg: Msg) => {
        this.setMsgMap(msg);
        this.setUserMsgList(msg);
        if (msg.head.type === Type.Ack) {
            this.setAckSet(msg);
        } else {
            this.setUnAckSet(msg);
        }
    }

    sendMsg = async (msg: Msg) => {
        this.newMsg(msg)
        await this.netConn?.send(msg);
    }

    setUserId = (userId: bigint) => {
        this.setState({ userId: userId });
    }

    setUserAvatar = (avatar: string) => {
        this.setState({ userAvatar: avatar });
    }

    setUserNickname = (nickname: string) => {
        this.setState({ userNickname: nickname });
    }

    setContactList = (list: Array<any>) => {
        this.setState({ contactList: list });
    }

    setCurrentChatPeerId = (peerId: bigint) => {
        let list = this.state.msgMap.get(peerId)
        if (list === undefined) {
            list = [];
            this.state.msgMap.set(peerId, list);
        }
        this.setState({ currentChatMsgList: [...list] });
        // @todo: avatar
        let currAvatar = "/src/assets/avatar/default-avatar-" + peerId + ".png";
        // @todo: remark
        let currRemark = "prim-user-" + peerId;
        this.setState({ currentChatPeerAvatar: currAvatar });
        this.setState({ currentChatPeerRemark: currRemark });
        this.setState({ currentChatPeerId: peerId });
        this.setUserMsgListItemUnread(peerId, false);
    }

    recvMsg = (msg: Msg) => {
        this.newMsg(msg);
    }

    saveMsg = async () => {
        let obj = await KVDB.get("saved-ack-map-" + this.state.userId);
        if (obj === undefined) {
            obj = {};
        }
        let savedAckMap0 = new Map<string, string>(Object.entries(obj));
        let savedAckMap = new Map<bigint, bigint>();
        savedAckMap0.forEach((value, key) => {
            savedAckMap.set(BigInt(key), BigInt(value));
        });
        this.setState({
            savedAckMap: savedAckMap,
        })
        setInterval(async () => {
            this.state.msgMap.forEach(async (value, key) => {
                let newest = this.state.savedAckMap.get(key);
                if (newest === undefined) {
                    newest = 0n;
                }
                let oldest = newest;
                for (let i = value.length - 1; i >= 0; --i) {
                    if (value[i].head.seqNum !== 0n) {
                        if (value[i].head.seqNum > newest) {
                            newest = value[i].head.seqNum;
                        }
                        if (value[i].head.seqNum <= oldest) {
                            break;
                        }
                        let slice = new Array<Msg>();
                        slice.push(value[i]);
                        await MsgDB.saveMsg(slice);
                    }
                }
                if (newest > oldest) {
                    let map = this.state.savedAckMap;
                    map.set(key, newest);
                    this.setState({ savedAckMap: map });
                }
            });
            await KVDB.set("saved-ack-map-" + this.state.userId, this.state.savedAckMap);
        }, 1000)
    }

    pullMsg = async () => {
        let inbox = await HttpClient.get("/message/inbox", {}, true);
        if (!inbox.ok) {
            console.log(inbox.errMsg);
            alert("unknown error")
            return;
        }
        let list = inbox.data as Array<string>;
        for (let i = 0; i < list.length; ++i) {
            let peerId = BigInt(list[i]);
            let oldSeqNum = this.state.savedAckMap.get(peerId);
            if (oldSeqNum === undefined) {
                oldSeqNum = 0n;
            }
            let newSeqNum = oldSeqNum + 100n;
            while (true) {
                oldSeqNum += 1n;
                newSeqNum = oldSeqNum + 100n;
                let resp = await HttpClient.get("/message/history", {
                    peer_id: peerId,
                    old_seq_num: oldSeqNum,
                    new_seq_num: newSeqNum,
                }, true);
                if (!resp.ok) {
                    console.log(resp.errMsg);
                    break;
                }
                let msgList = resp.data as Array<any>;
                if (msgList.length === 0) {
                    break;
                }
                oldSeqNum = oldSeqNum + BigInt(msgList.length);
                for (let j = 0; j < msgList.length; ++j) {
                    let body = msgList[j] as Array<number>;
                    let buffer = new Uint8Array(body.length);
                    for (let k = 0; k < body.length; ++k) {
                        buffer[k] = body[k];
                    }
                    let msg = Msg.fromArrayBuffer(buffer.buffer);
                    this.newMsg(msg);
                }
            }
        }
    }

    unreadUpdate = async () => {
        let list = new Array<UserMsgListItemData>();
        this.state.userMsgList.forEach(async (value) => {
            let resp = await HttpClient.get("/message/unread", {
                peer_id: value.peerId,
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
                return;
            }
            let unreadSeqNum = BigInt(resp.data);
            let listList = this.state.msgMap.get(value.peerId);
            if (listList === undefined) {
                return;
            }
            let last = listList[listList.length - 1];
            if (last === undefined) {
                return;
            }
            let item = value;
            if (unreadSeqNum > last.head.seqNum) {
                item.unreadNumber = Number(unreadSeqNum - last.head.seqNum);
            }
            list.push(item);
        });
        this.setState({ userMsgList: list });
    }

    setup = async () => {
        let token = await KVDB.get("access-token");
        let userId = await KVDB.get("user-id");
        if (token === undefined || userId === undefined) {
            this.state.loginRedirect();
            return;
        } else {
            let resp = await HttpClient.put('/user', {}, {}, true);
            if (!resp.ok) {
                this.state.loginRedirect();
                return;
            }
        }
        let resp = (await HttpClient.get("/which_address", {}, true))
        if (!resp.ok) {
            alert("unknown error")
            // this.state.loginRedirect();
            return;
        }
        let address = resp.data as string;
        console.log(address);
        // @todo mode switch
        this.netConn = new Client(address, token as string, "udp", BigInt(userId), 0, this.recvMsg);
        await this.netConn.connect();
        await this.pullMsg();
        await this.saveMsg();
        await this.unreadUpdate();
    }

    componentDidMount = async () => {
        await this.setup();
        let count = 1;
        const f = () => {
            if (count > 20) {
                return;
            }
            ++count;
            setTimeout(() => {
                this.newMsg(randomMsg());
                f();
            }, 500);
        }
        // f();
    }

    disconnect = async () => {
        if (this.netConn !== undefined) {
            await this.netConn.disconnect();
        }
    }

    componentWillUnmount = async () => {
        await this.disconnect();
    }

    render(): ReactNode {
        return (
            <div id={"root"}>
                <GlobalContext.Provider value={{
                    userMsgList: this.state.userMsgList,
                    contactList: this.state.contactList,
                    userId: this.state.userId,
                    userAvatar: this.state.userAvatar,
                    userNickname: this.state.userNickname,
                    nodeId: this.state.nodeId,
                    currentChatMsgList: this.state.currentChatMsgList,
                    currentChatPeerId: this.state.currentChatPeerId,
                    currentChatPeerAvatar: this.state.currentChatPeerAvatar,
                    currentChatPeerRemark: this.state.currentChatPeerRemark,
                    unAckSet: this.state.unAckSet,
                    setUserId: this.setUserId,
                    setUserAvatar: this.setUserAvatar,
                    setUserNickname: this.setUserNickname,
                    setContactList: this.setContactList,
                    setCurrentChatPeerId: this.setCurrentChatPeerId,
                    sendMsg: this.sendMsg,
                    setUnread: this.setUserMsgListItemUnread,
                    setLoginPageDirect: this.setLoginRedirect,
                    setup: this.setup,
                    disconnect: this.disconnect,
                    clearState: this.clearState,
                }}>
                    <BrowserRouter>
                        <Routes>
                            <Route path="/login" element={<Login></Login>} />
                            <Route path="/" element={<Chat></Chat>} />
                            <Route path="/contacts" element={<Contacts></Contacts>} />
                            <Route path="/more" element={<More></More>} />
                        </Routes>
                    </BrowserRouter>
                </GlobalContext.Provider>
            </div>
        )
    }
}

export default App;