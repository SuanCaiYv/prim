import { BrowserRouter, Route, Routes } from "react-router-dom";
import Chat from "./components/chat/Chat";
import Contacts from "./components/contacts/Contacts";
import More from "./components/more/More";
import './App.css'
import { Context, GlobalContext } from "./context/GlobalContext";
import { ReactNode, useState } from "react";
import { Msg } from "./entity/msg";
import React from "react";

class Props {}

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

    setUserMsgList(list: Array<Msg>) {
        console.log("updateUserMsgList", list.length);
        this.setState((state) => {
            state.userMsgList = list
        });
    }

    setMsgMap(map: Map<string, Msg[]>) {
        this.setState({ msgMap: map });
    }

    setContactList(list: Array<string>) {
        this.setState({ contactList: list });
    }

    setUserId(id: bigint) {
        this.setState({ userId: id });
    }

    setUserAvatar(avatar: string) {
        this.setState({ userAvatar: avatar });
    }

    setUserNickname(nickname: string) {
        this.setState({ userNickname: nickname });
    }

    componentDidMount() {
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
                    setUserMsgList: (list: Array<Msg>) => this.setUserMsgList(list),
                    setMsgMap: (map: Map<string, Msg[]>) => this.setMsgMap(map),
                    setContactList: (list: Array<string>) => this.setContactList(list),
                    setUserId: (id: bigint) => this.setUserId(id),
                    setUserAvatar: (avatar: string) => this.setUserAvatar(avatar),
                    setUserNickname: (nickname: string) => this.setUserNickname(nickname)
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