import React from "react";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { GROUP_ID_THRESHOLD, Msg, Type } from "../../entity/msg";
import { UserInfo } from "../../service/user/userInfo";
import "./MsgListItem.css";
import AddFriend from "./special/AddFriend";

const MsgListItem = (props: {
    userId: bigint;
    rawMsg: Msg;
}) => {
    let [avatar, setAvatar] = React.useState<string>("");
    let [remark, setRemark] = React.useState<string>("");
    let [content, setContent] = React.useState<any>();
    let context = React.useContext(GlobalContext) as Context;

    React.useEffect(() => {
        (async () => {
            if (props.userId === context.userId) {
                let [avatar, _] = await UserInfo.avatarNickname(context.userId);
                setAvatar(avatar);
            } else {
                let [avatar, _] = await UserInfo.avatarRemark(context.userId, context.currentChatPeerId);
                setAvatar(avatar);
            }
            if (props.rawMsg.head.sender >= GROUP_ID_THRESHOLD || props.rawMsg.head.receiver >= GROUP_ID_THRESHOLD) {
                let realSender = BigInt(props.rawMsg.extensionText());
                let [avatar, remark] = await UserInfo.avatarNickname(realSender);
                setRemark(remark);
                setAvatar(avatar);
            }
            if (props.rawMsg.head.type === Type.AddFriend) {
                let msg = props.rawMsg;
                if (msg.extension.byteLength === 0) {
                    if (msg.head.sender === context.userId) {
                        setContent('Waiting For Approval...');
                        return;
                    }
                    let [avatar, nickname] = await UserInfo.avatarNickname(props.userId);
                    setAvatar(avatar);
                    setContent(<AddFriend remark={msg.payloadText()} nickname={nickname} peerId={props.userId} />);
                } else {
                    let res = new TextDecoder().decode(msg.extension);
                    if (res === 'true') {
                        setContent('Hi! I am your friend now!');
                    } else {
                        setContent('I am sorry that I can not add you as my friend.');
                    }
                }
            } else {
                setContent(props.rawMsg.payloadText());
            }
        })();
    }, [])

    let key = props.userId + "-" + context.currentChatPeerId + "-" + props.rawMsg.head.timestamp;
    return (
        props.userId === context.userId ? (
            <div className={'msg-list-item-right'}>
                <div className={'item-content-right'}>
                    {
                        remark !== '' ? (
                            <div className={'remark-right'}>
                                <div className={'remark-right-text'}>
                                    {
                                        remark
                                    }
                                </div>
                            </div>
                        ) : (
                            null
                        )
                    }
                    <div className={'content-right'}>
                        {
                            content
                        }
                    </div>
                    <span className={'waiting-block'}>
                        {
                            context.unAckSet.has(key) ? 'X' : ''
                        }
                    </span>
                </div>
                <img className={'item-avatar'} src={avatar} alt="" />
            </div>
        ) : (
            <div className={'msg-list-item-left'}>
                <img className={'item-avatar'} src={avatar} alt="" />
                <div className={'item-content-left'}>
                    {
                        remark !== '' ? (
                            <div className={'remark-left'}>
                                <div className={'remark-left-text'}>
                                    {
                                        remark
                                    }
                                </div>
                            </div>
                        ) : (
                            null
                        )
                    }
                    <div className={'content-left'}>
                        {
                            content
                        }
                    </div>
                </div>
            </div>
        )
    )
}

export default MsgListItem