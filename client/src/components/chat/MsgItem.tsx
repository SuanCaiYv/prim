import React, { ReactNode } from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import "./MsgItem.css";

class Props {
    content: string = "";
    accountId: bigint = 0n;
    avatar: string = "";
    timestamp: bigint = 0n;
    remark: string = "";
    onClick: (accountId: bigint) => void = (accountId: bigint) => { };
}

class State { }

class MsgListItem extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    render(): ReactNode {
        let context = this.context as Context;
        return (
            this.props.accountId === context.userId ? (
                <div className="msg-list-item-right">
                    <img className="item-avatar" src={this.props.avatar} alt="" />
                    <div className="item-content-right">
                        <p className="content-right">
                            {
                                this.props.content
                            }
                        </p>
                    </div>
                </div>
            ) : (
                <div className="msg-list-item-left">
                    <img className="item-avatar" src={this.props.avatar} alt="" />
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