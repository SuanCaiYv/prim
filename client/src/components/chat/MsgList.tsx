import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgItem";
import './MsgList.css';

class MsgList extends React.Component {
    static contextType = GlobalContext;

    onClick = (accountId: bigint) => { }

    render(): React.ReactNode {
        let context = this.context as Context;
        console.log(context.currentChatMsgList);
        return (
            <div className="msg-list">
                {
                    context.currentChatMsgList.map((msg, index) => {
                        let avatar = '/src/assets/avatar/default-avatar-' + msg.head.sender + '.png';
                        let remark = 'prim-user-' + msg.head.sender;
                        return <MsgListItem key={index} content={msg.payloadText()} accountId={msg.head.sender} avatar={avatar} timestamp={msg.head.timestamp} remark={remark} onClick={this.onClick}></MsgListItem>
                    })
                }
            </div>
        )
    }
}

export default MsgList