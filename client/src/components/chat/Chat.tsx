import React from 'react';
import { ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { Context, GlobalContext } from '../../context/GlobalContext';
import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Chat.css';
import UserMsgListItem from './UserMsgListItem';

class Props {}

class State {
    context: Context = new Context();
}

class Chat extends React.Component<Props, State> {
    static contextType = GlobalContext;
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    setContext = (context: Context) => {
        this.setState({
            context: context
        });
    }

    onUserMsgListItemClick = (peerId: bigint) => {
        let context = this.context as Context;
        context.setCurrentChatUserId(peerId);
    }

    componentDidMount() {
        let context = this.context as Context;
        this.setState({
            context: context
        });
    }
    render(): ReactNode {
        let context = this.context as Context;
        return (
            <div className="chat">
                <Layout>
                    <Header clicked='chat'></Header>
                    <List>
                        {
                            context.userMsgList.reverse().map((msg, index) => {
                                return <UserMsgListItem key={index} msg={msg.text} peerId={msg.peerId} avatar={msg.avatar} timestamp={msg.timestamp} number={msg.unreadNumber} remark={msg.remark} onClick={this.onUserMsgListItemClick}></UserMsgListItem>
                            })
                        }
                    </List>
                    <Main></Main>
                </Layout>
            </div>
        )
    }
}

export default Chat;