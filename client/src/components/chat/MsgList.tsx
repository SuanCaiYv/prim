import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import MsgListItem from "./MsgItem";
import './MsgList.css';

class MsgList extends React.Component {
    listRef = React.createRef<HTMLDivElement>();

    static contextType = GlobalContext;

    onClick = (accountId: bigint) => { }

    componentDidUpdate(): void {
        if (this.listRef.current) {
            this.listRef.current.scrollTop = this.listRef.current.scrollHeight;
          }
    }

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="msg-list" ref={this.listRef}>
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