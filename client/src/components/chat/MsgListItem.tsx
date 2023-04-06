import React, { ReactNode } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { GROUP_ID_THRESHOLD, Msg, Type } from "../../entity/msg";
import { UserInfo } from "../../service/user/userInfo";
import "./MsgListItem.css";
import AddFriend from "./special/AddFriend";

class Props {
    accountId: bigint = 0n;
    rawMsg: Msg = Msg.text(0n, 0n, 0, "");
}

class State {
    avatar: string = "";
    remark: string = "";
    content: any;
}

class MsgListItem extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    componentDidMount = async () => {
        let context = this.context as Context;
        if (this.props.accountId === context.userId) {
            let [avatar, _] = await UserInfo.avatarNickname(context.userId);
            this.setState({
                avatar: avatar
            })
        } else {
            let [avatar, _] = await UserInfo.avatarRemark(context.userId, context.currentChatPeerId);
            this.setState({
                avatar: avatar,
            })
        }
        if (this.props.rawMsg.head.sender >= GROUP_ID_THRESHOLD || this.props.rawMsg.head.receiver >= GROUP_ID_THRESHOLD) {
            let realSender = BigInt(this.props.rawMsg.extensionText());
            let [avatar, remark] = await UserInfo.avatarNickname(realSender);
            this.setState({
                avatar: avatar,
                remark: remark
            })
        }
        if (this.props.rawMsg.head.type === Type.AddFriend) {
            let msg = this.props.rawMsg;
            if (msg.extension.byteLength === 0) {
                if (msg.head.sender === context.userId) {
                    this.setState({
                        content: 'Waiting For Approval...'
                    });
                    return;
                }
                let [avatar, nickname] = await UserInfo.avatarNickname(this.props.accountId);
                this.setState({
                    avatar: avatar,
                    content: <AddFriend remark={msg.payloadText()} nickname={nickname} peerId={this.props.accountId} />
                })
            } else {
                let res = new TextDecoder().decode(msg.extension);
                if (res === 'true') {
                    this.setState({
                        content: 'Hi! I am your friend now!'
                    })
                } else {
                    this.setState({
                        content: 'I am sorry that I can not add you as my friend.'
                    })
                }
            }
        } else {
            this.setState({
                content: this.props.rawMsg.payloadText()
            })
        }
    }

    render = (): ReactNode => {
        let context = this.context as Context;
        let key = this.props.accountId + "-" + context.currentChatPeerId + "-" + this.props.rawMsg.head.timestamp;
        return (
            this.props.accountId === context.userId ? (
                <div className="msg-list-item-right">
                    <div className="item-content-right">
                        {
                            this.state.remark !== '' ? (
                                <div className="remark-right">
                                    <div className="remark-right-text">
                                        {
                                            this.state.remark
                                        }
                                    </div>
                                </div>
                            ) : (
                                <div></div>
                            )
                        }
                        <div className="content-right">
                            {
                                this.state.content
                            }
                        </div>
                        <span className="waiting-block">
                            {
                                context.unAckSet.has(key) ? 'X' : ''
                            }
                        </span>
                    </div>
                    <img className="item-avatar" src={this.state.avatar} alt="" />
                </div>
            ) : (
                <div className="msg-list-item-left">
                    <img className="item-avatar" src={this.state.avatar} alt="" />
                    <div className="item-content-left">
                        {
                            this.state.remark !== '' ? (
                                <div className="remark-left">
                                    <div className="remark-left-text">
                                        {
                                            this.state.remark
                                        }
                                    </div>
                                </div>
                            ) : (
                                <div></div>
                            )
                        }
                        <div className="content-left">
                            {
                                this.state.content
                            }
                        </div>
                    </div>
                </div>
            )
        )
    }
}

export default MsgListItem