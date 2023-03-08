import { BrowserRouter, Route, Routes } from "react-router-dom";
import Chat from "./components/chat/Chat";
import Contacts from "./components/contacts/Contacts";
import More from "./components/more/More";
import './App.css'
import { GlobalContext, UserMsgListItemData } from "./context/GlobalContext";
import { ReactNode, useState } from "react";
import { Msg } from "./entity/msg";
import React from "react";
import { randomMsg } from "./mock/chat";

class Props { }

class State {
    test: Map<number, number[]> = new Map();
    userMsgList: Array<UserMsgListItemData> = [];
    msgMap: Map<bigint, Msg[]> = new Map();
    contactList: Array<any> = [];
    userId: bigint = 0n;
    userAvatar: string = "";
    userNickname: string = "prim-user";
    currentChatMsgList: Array<Msg> = [];
    currentChatUserId: bigint = 0n;
}

class App extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    peerId = (id1: bigint, id2: bigint) => {
        if (this.state.userId === id1) {
            return id2;
        } else {
            return id1;
        }
    }

    setUserMsgList = (msg: Msg) => {
        let peerId = this.peerId(msg.head.sender, msg.head.receiver);
        let text = msg.payloadText();
        let timestamp = msg.head.timestamp;
        // @todo: avatar
        let avatar = "/src/assets/avatar/default-avatar-" + peerId + ".png";
        // @todo: nickname
        let nickname = "prim-user-" + peerId;
        let number = 1;
        let list = this.state.userMsgList;
        let newList
        let v = list.find((item) => {
            return item.peerId === peerId;
        });
        if (v !== undefined) {
            number = v.unreadNumber + 1;
            newList = [...list.filter((item) => {
                return item.peerId !== peerId;
            }), new UserMsgListItemData(peerId, avatar, nickname, text, timestamp, number)]
        } else {
            newList = [...list, new UserMsgListItemData(peerId, avatar, nickname, text, timestamp, number)];
        }
        this.setState({ userMsgList: newList });
    }

    setMsgMap = (msg: Msg) => {
        let peerId = this.peerId(msg.head.sender, msg.head.receiver);
        let map = this.state.msgMap;
        let list = map.get(peerId);
        if (list === undefined) {
            map.set(peerId, [msg]);
        } else {
            list.push(msg);
        }
        // @todo resort
        if (peerId === this.state.currentChatUserId) {
            this.setState({ currentChatMsgList: [...this.state.currentChatMsgList, msg] });
        }
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

    setCurrentChatUserId = (userId: bigint) => {
        let list = this.state.msgMap.get(userId)
        if (list === undefined) {
            list = [];
            this.state.msgMap.set(userId, list);
        }
        this.setState({ currentChatMsgList: list });
        this.setState({ currentChatUserId: userId });
    }

    componentDidMount() {
        let count = 1;
        const f = () => {
            if (count > 20) {
                return;
            }
            ++count;
            setTimeout(() => {
                this.setUserMsgList(randomMsg());
                f();
            }, 2000);
        }
        f()
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
                    currentChatMsgList: this.state.currentChatMsgList,
                    currentChatUserId: this.state.currentChatUserId,
                    setUserMsgList: this.setUserMsgList,
                    setMsgMap: this.setMsgMap,
                    setUserId: this.setUserId,
                    setUserAvatar: this.setUserAvatar,
                    setUserNickname: this.setUserNickname,
                    setContactList: this.setContactList,
                    setCurrentChatUserId: this.setCurrentChatUserId
                }}>
                    <BrowserRouter>
                        <Routes>
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