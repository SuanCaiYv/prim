import React, { useEffect, useRef } from "react";
import './ChatHeader.css';
import { Context, GlobalContext } from "../../context/GlobalContext";
import { UserInfo } from "../../service/user/userInfo";
import { GROUP_ID_THRESHOLD } from "../../entity/msg";
import { useNavigate } from "react-router-dom";
import { HttpClient } from "../../net/http";
import { alertComponentNormal } from "../portal/Portal";

const UpdateGroupInfo = () => {
    let context = React.useContext(GlobalContext) as Context;
    let [avatar, setAvatar] = React.useState("");
    let [name, setName] = React.useState("");
    let [announcement, setAnnouncement] = React.useState("");

    const onAvatarChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        let file = e.target.files![0];
        let reader = new FileReader();
        reader.onload = async (e) => {
            let avatar = e.target!.result as string;
            let resp = await HttpClient.put('/user/avatar', {}, {
                avatar: avatar
            }, true);
            if (resp.ok) {
                setAvatar(avatar);
                // context.updateAvatar(avatar);
            } else {
                console.log(resp.errMsg);
            }
        }
        reader.readAsDataURL(file);
    }

    const onNameChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        let name = e.target.value;
        setName(name);
    }

    const onAnnouncementChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        let announcement = e.target.value;
        setAnnouncement(announcement);
    }

    useEffect(() => {
        (async () => {
            let [avatar, name, announcement] = await UserInfo.groupInfo(context.currentChatPeerId);
            setAvatar(avatar);
            setName(name);
            setAnnouncement(announcement);
        })();
        return () => { };
    }, []);

    return (
        <div className={'update-group-info'}>
            <div className={'update-group-info-avatar'}>
                <label htmlFor='contact-avatar'>
                    <img src={avatar} alt="" />
                </label>
                <input type='file' id='contact-avatar' hidden onChange={onAvatarChange}/>
            </div>
            <div className={'update-group-info-text mt-4'}>
                <span>Name</span>
                <input type="text" value={name} onChange={onNameChange} autoCorrect="off"/>
            </div>
            <div className={'update-group-info-text'}>
                <span>Announcement</span>
                <input type="text" value={announcement} onChange={onAnnouncementChange} autoCorrect="off" />
            </div>
        </div>
    )
}

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

    const infoClick = () => {
        alertComponentNormal(<UpdateGroupInfo />);
    }

    return (
        <div className={'chat-header'}>
            <div className={'chat-header-remark'} onClick={infoClick}>{remark}</div>
            <div className={'chat-header-show-info'}>
                <button onClick={onClick}>Info</button>
            </div>
        </div>
    )
}

export default ChatHeader