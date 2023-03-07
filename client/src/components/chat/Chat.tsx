import React from 'react';
import { ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { randomMsg } from '../../mock/chat';
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
                            context.userMsgList.map((msg, index) => {
                                return <UserMsgListItem key={index} msg={msg.payloadText()} avatar='src/assets/avatar/default-avatar-1.png' timestamp={msg.head.timestamp} number={99}></UserMsgListItem>
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