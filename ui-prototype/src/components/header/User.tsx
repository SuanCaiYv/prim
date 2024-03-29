import { useContext, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { UserInfo } from '../../service/user/userInfo';
import './User.css'

export default function User() {
    let context = useContext(GlobalContext) as Context;
    let [avatar, setAvatar] = useState("");

    useEffect(() => {
        (async () => {
            let [avatar, _nickname] = await UserInfo.avatarNickname(context.userId);
            setAvatar(avatar);
        })();
    });

    return (
        <div className={'user'} data-tauri-drag-region>
            <Link to="/contacts" onClick={async () => {
                context.setCurrentContactUserId(context.userId);
            }}>
                <img className={'user'} src={avatar} alt="" />
            </Link>
        </div>
    )
}