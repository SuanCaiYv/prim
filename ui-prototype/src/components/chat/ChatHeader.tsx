import React from "react";
import './ChatHeader.css';
import { Context, GlobalContext } from "../../context/GlobalContext";
import { UserInfo } from "../../service/user/userInfo";

class State {
    remark: string = "";
}

class Props {}

class ChatHeader extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    async componentDidMount() {
        let context = this.context as Context;
        let [_avatar, remark] = await UserInfo.avatarRemark(context.userId, context.currentChatPeerId);
        this.setState({
            remark: remark
        })
    }

    render = (): React.ReactNode => {
        return (
            <div className="chat-header">
                <div className="chat-header-remark">
                    {
                        this.state.remark
                    }
                </div>
            </div>
        )
    }
}

export default ChatHeader