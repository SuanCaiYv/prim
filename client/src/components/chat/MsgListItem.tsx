import React, { ReactNode } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { UserInfo } from "../../service/user/userInfo";
import "./MsgListItem.css";

class Props {
    content: any;
    accountId: bigint = 0n;
    timestamp: bigint = 0n;
}

class State {
    avatar: string = "";
    remark: string = "";
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
            let [avatar, remark] = await UserInfo.avatarRemark(context.userId, context.currentChatPeerId);
            this.setState({
                avatar: avatar,
                remark: remark
            })
        }
    }

    render(): ReactNode {
        let context = this.context as Context;
        let key = this.props.accountId + "-" + context.currentChatPeerId + "-" + this.props.timestamp;
        return (
            this.props.accountId === context.userId ? (
                <div className="msg-list-item-right">
                    <div className="item-content-right">
                        <div className="content-right">
                            {
                                this.props.content
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
                        <div className="content-left">
                            {
                                this.props.content
                            }
                        </div>
                    </div>
                </div>
            )
        )
    }
}

export default MsgListItem