import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { Msg } from "../../entity/msg";
import { HttpClient } from "../../net/http";
import { UserInfo } from "../../service/user/userInfo";
import './InputArea.css';

class Props { }

class State {
    value: string = "";
}

class InputArea extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    handleKeyDown = async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            let value = this.state.value
            if (value.length > 0) {
                if (value.endsWith('\n')) {
                    value = value.substring(0, value.length - 1);
                }
                let context = this.context as Context;
                let nodeId = await UserInfo.whichNode(context.currentChatPeerId);
                let msg = Msg.text(context.userId, context.currentChatPeerId, nodeId, value);
                await context.sendMsg(msg);
                this.setState({ value: "" });
                await this.onClick();
            }
        }
    }

    onChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        this.setState({ value: e.target.value });
    }

    onClick = async () => {
        let context = this.context as Context;
        await context.setUnread(context.currentChatPeerId, false)
        let msgList = context.msgMap.get(context.currentChatPeerId);
        if (msgList !== undefined && msgList.length > 0) {
            let seqNum = msgList[msgList.length - 1].head.seqNum;
            await HttpClient.put('/message/unread', {
                last_read_seq: seqNum,
                peer_id: context.currentChatPeerId
            }, {}, true);
        }
    }

    render(): React.ReactNode {
        return (
            <div className="input-area">
                <textarea className="text-area" value={this.state.value} onChange={this.onChange} onKeyDown={this.handleKeyDown} onClick={this.onClick} autoComplete="off" autoCorrect="off"></textarea>
            </div>
        )
    }
}

export default InputArea