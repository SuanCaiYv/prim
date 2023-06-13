import React, { useRef } from "react";
import './ChatHeader.css';
import { Context, GlobalContext } from "../../context/GlobalContext";
import { UserInfo } from "../../service/user/userInfo";
import { GROUP_ID_THRESHOLD } from "../../entity/msg";
import { useNavigate } from "react-router-dom";

const ChatHeader = (props: {
    setShowInfo: (showInfo: boolean) => void
}) => {
    let [remark, setRemark] = React.useState("");
    let show = useRef(false);
    let context = React.useContext(GlobalContext) as Context;
    let navigate = useNavigate();

    React.useEffect(() => {
        show.current = false;
        UserInfo.avatarRemark(context.userId, context.currentChatPeerId).then(([_avatar, remark]) => {
            setRemark(remark);
        })
    }, [context.currentChatPeerId]);

    const onClick = () => {
        if (context.currentChatPeerId >= GROUP_ID_THRESHOLD) {
            show.current = !show.current;
            props.setShowInfo(show.current);
        } else {
            context.setCurrentContactUserId(context.currentChatPeerId);
            navigate('/contacts');
        }
    }

    return (
        <div className={'chat-header'}>
            <div className={'chat-header-remark'}>
                {
                    remark
                }
            </div>
            <div className={'chat-header-show-info'}>
                <button onClick={onClick}>Info</button>
            </div>
        </div>
    )
}

export default ChatHeader