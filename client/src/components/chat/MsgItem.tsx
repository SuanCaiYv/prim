import React, { ReactNode } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { KVDB } from "../../service/database";
import { UserInfo } from "../../service/user/userInfo";
import "./MsgItem.css";

class Props {
    content: string = "";
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
                        <p className="content-right">
                            {
                                this.props.content
                            }
                        </p>
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
                        <p className="content-left">
                            {
                                this.props.content
                            }
                        </p>
                    </div>
                </div>
            )
        )
    }
}

export default MsgListItem