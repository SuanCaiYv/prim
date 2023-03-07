import { BrowserRouter, Route, Routes } from "react-router-dom";
import Chat from "./components/chat/Chat";
import Contacts from "./components/contacts/Contacts";
import More from "./components/more/More";
import './App.css'
import { Context, GlobalContext } from "./context/GlobalContext";
import { ReactNode, useState } from "react";
import { Msg } from "./entity/msg";
import React from "react";
import { randomMsg } from "./mock/chat";

class Props { }

class State {
    userMsgList: Array<Msg> = [];
    msgMap: Map<string, Msg[]> = new Map();
    contactList: Array<string> = [];
    userId: bigint = 0n;
    userAvatar: string = "";
    userNickname: string = "prim-user";
}

class App extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    setUserMsgList = (list: Array<Msg>, ...cb: (() => void)[]) => {
        this.setState({ userMsgList: list }, () => {
            if (cb.length > 0) {
                cb[0]();
            }
        });
    }

    setMsgMap = (map: Map<string, Msg[]>, ...cb: (() => void)[]) => {
        this.setState({ msgMap: map }, () => {
            if (cb.length > 0) {
                cb[0]();
            }
        });
    }

    setContactList = (list: Array<string>, ...cb: (() => void)[]) => {
        this.setState({ contactList: list });
    }

    setUserId = (id: bigint, ...cb: (() => void)[]) => {
        this.setState({ userId: id });
    }

    setUserAvatar = (avatar: string, ...cb: (() => void)[]) => {
        this.setState({ userAvatar: avatar });
    }

    setUserNickname = (nickname: string, ...cb: (() => void)[]) => {
        this.setState({ userNickname: nickname });
    }

    componentDidMount() {
        let count = 0;
        const f = () => {
            if (count > 1) {
                return;
            }
            ++count;
            setTimeout(() => {
                this.setUserMsgList([...this.state.userMsgList, randomMsg()], f);
            }, 1000);
        }
        f()
    }

    render(): ReactNode {
        return (
            <div id={"root"}>
                <GlobalContext.Provider value={{
                    userMsgList: this.state.userMsgList,
                    msgMap: this.state.msgMap,
                    contactList: this.state.contactList,
                    userId: this.state.userId,
                    userAvatar: this.state.userAvatar,
                    userNickname: this.state.userNickname,
                    setUserMsgList: (list: Array<Msg>, ...cb: (() => void)[]) => this.setUserMsgList(list, ...cb),
                    setMsgMap: (map: Map<string, Msg[]>, ...cb: (() => void)[]) => this.setMsgMap(map, ...cb),
                    setContactList: (list: Array<string>, ...cb: (() => void)[]) => this.setContactList(list, ...cb),
                    setUserId: (id: bigint, ...cb: (() => void)[]) => this.setUserId(id, ...cb),
                    setUserAvatar: (avatar: string, ...cb: (() => void)[]) => this.setUserAvatar(avatar, ...cb),
                    setUserNickname: (nickname: string, ...cb: (() => void)[]) => this.setUserNickname(nickname, ...cb)
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