import React, { useEffect } from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { HttpClient } from '../../net/http';
import { KVDB } from '../../service/database';
import { UserInfo } from '../../service/user/userInfo';
import './ContactInfo.css'

const ContactInfo = () => {
    let context = React.useContext(GlobalContext) as Context;
    let [avatar, setAvatar] = React.useState("");
    let [nickname, setNickname] = React.useState("");
    let [signature, setSignature] = React.useState("");
    let [remark, setRemark] = React.useState("");
    let [userId, setUserId] = React.useState<bigint>(0n);

    useEffect(() => {
        setUserId(context.currentContactUserId);
        (async () => {
            let userInfo = await HttpClient.get('/user/info', {
                peer_id: Number(context.currentContactUserId)
            }, true)
            if (!userInfo.ok) {
                console.log(userInfo.errMsg);
                return;
            }
            if (context.currentContactUserId !== context.userId) {
                let [_, remark] = await UserInfo.avatarRemark(context.userId, context.currentContactUserId);
                let [_avatar, nickname] = await UserInfo.avatarNickname(context.currentContactUserId);
                if (remark === '') {
                    remark = nickname;
                }
                setRemark(remark);
            }
            setAvatar(userInfo.data.avatar);
            setNickname(userInfo.data.nickname);
            setSignature(userInfo.data.signature);
        })();
        return () => { };
    }, [userId]);

    const onSignatureChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        setSignature(e.target.value);
    }

    const onNicknameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setNickname(e.target.value);
    }

    const onRemarkChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setRemark(e.target.value);
    }

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

    const onSignatureKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (signature === '') {
                return;
            }
            let resp = await HttpClient.put('/user/info', {}, {
                signature: signature
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-s-i')!.blur();
        }
    }

    const onNicknameKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (nickname === '') {
                return;
            }
            let resp = await HttpClient.put('/user/info', {}, {
                nickname: nickname
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-n-i')!.blur();
        }
    }

    const onRemarkKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (remark === '') {
                return;
            }
            let resp = await HttpClient.put('/relationship/friend', {}, {
                peer_id: Number(context.currentContactUserId),
                remark: remark
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-r-i')!.blur();
        }
    }

    const onLogout = async () => {
        await KVDB.del('access-token');
        await context.disconnect();
        context.clearState();
    }

    return (
        <div className={'contact-info-main'}>
            <div className={'na1'}></div>
            <div className={'contact-info-avatar'}>
                <label htmlFor='contact-avatar'>
                    <img src={avatar} alt="" />
                </label>
                {
                    context.currentContactUserId === context.userId &&
                    <input type='file' id='contact-avatar' hidden onChange={onAvatarChange} />
                }
            </div>
            <div className="contact-info-account-id">
                <div className={'info-tag'}>
                    <img src="/assets/id.png" alt="" />
                </div>
                <div className={'info-body'}>
                    {
                        context.currentContactUserId + ""
                    }
                </div>
            </div>
            <div className="contact-info-logout">
                {
                    context.currentContactUserId === context.userId &&
                    <Link to="/sign">
                        <button className="logout-btn" onClick={onLogout}>Logout</button>
                    </Link>
                }
            </div>
            <div className="contact-info-signature">
                <div className={'info-tag'}>
                    <img src="/assets/signature.png" alt="" />
                </div>
                <div className={'info-body'}>
                    {
                        context.currentContactUserId !== context.userId ?
                            signature :
                            <input id='c-i-s-i' className='c-i-input' type="text" value={signature}
                                placeholder='Say something to make a different self!'
                                onChange={onSignatureChange} onKeyDown={onSignatureKeyDown} autoCorrect='off' />
                    }
                </div>
            </div>
            <div className="contact-info-nickname">
                <div className={'info-tag'}>
                    <img src="/assets/nickname.png" alt="" />
                </div>
                <div className={'info-body'}>
                    {
                        context.currentContactUserId === context.userId
                            ?
                            <input id='c-i-n-i' className='c-i-input' type="text"
                                value={nickname} placeholder='nickname'
                                onChange={onNicknameChange} onKeyDown={onNicknameKeyDown} autoCorrect='off' />
                            :
                            nickname
                    }
                </div>
            </div>
            <div className="contact-info-remark">
                <div className={'info-tag'}>
                    <img src="/assets/remark.png" alt="" />
                </div>
                <div className={'info-body'}>
                    {
                        context.currentContactUserId !== context.userId &&
                        <input id='c-i-r-i' className='c-i-input' type="text"
                            value={remark} placeholder='remark'
                            onChange={onRemarkChange} onKeyDown={onRemarkKeyDown} autoCorrect='off' />
                    }
                </div>
            </div>
            <div className={'na2'}></div>
            <div className={'na3'}></div>
            <div className={'na4'}></div>
            <div className={'na5'}></div>
            <div className={'na6'}></div>
        </div>
    )
}

export default ContactInfo;