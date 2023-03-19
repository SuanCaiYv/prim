import React from 'react';
import { ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Chat.css';
import ChatHeader from './ChatHeader';
import InputArea from './InputArea';
import MsgList from './MsgList';
import UserMsgListItem from './UserMsgListItem';

class Props { }

class State { }

class Chat extends React.Component<Props, State> {
    static contextType = GlobalContext;

    loginARef: React.RefObject<any>;
    constructor(props: any) {
        super(props);
        this.state = new State();
        this.loginARef = React.createRef();
    }

    loginARefClick = () => {
        this.loginARef.current.click();
    }

    onUserMsgListItemClick = (peerId: bigint) => {
        let context = this.context as Context;
        context.setCurrentChatPeerId(peerId);
    }

    clearChatArea = () => {
        // todo
    }

    componentDidMount = async () => {
        let context = this.context as Context;
        this.setState({
            context: context
        });
        context.setLoginPageDirect(this.loginARefClick);
    }
    render(): ReactNode {
        let context = this.context as Context;
        return (
            <div className="chat">
                <Layout>
                    <Header clicked='chat'></Header>
                    <List clearChatArea={this.clearChatArea}>
                        {
                            context.userMsgList.map((msg, index) => {
                                return <UserMsgListItem key={index} msg={msg.text} peerId={msg.peerId} avatar={msg.avatar} timestamp={msg.timestamp} number={msg.unreadNumber} remark={msg.remark} onClick={this.onUserMsgListItemClick}></UserMsgListItem>
                            })
                        }
                    </List>
                    <Main>
                        {
                            context.currentChatPeerId === 0n ? (
                                <div></div>
                            ) : (
                                <div className="chat-main">
                                    <ChatHeader></ChatHeader>
                                    <MsgList></MsgList>
                                    <InputArea></InputArea>
                                </div>
                            )
                        }
                    </Main>
                    <Link to="/login" className="login-a-direct" ref={this.loginARef}></Link>
                </Layout>
            </div>
        )
    }
}

export default Chat;