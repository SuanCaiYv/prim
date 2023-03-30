import React from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { HttpClient } from '../../net/http';
import { KVDB } from '../../service/database';
import { UserInfo } from '../../service/user/userInfo';
import './ContactInfo.css'

class Props { }

class State {
    avatar: string = "";
    nickname: string = "";
    signature: string = "";
    remark: string = "";
    userId: bigint = 0n;
}

class ContactInfo extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: Props) {
        super(props);
        this.state = new State();
    }

    componentDidMount = async () => {
        let context = this.context as Context;
        this.setState({
            userId: context.currentContactUserId
        });
        let userInfo = await HttpClient.get('/user/info', {
            peer_id: Number(context.currentContactUserId)
        }, true)
        if (!userInfo.ok) {
            console.log(userInfo.errMsg);
            return;
        }
        if (context.currentContactUserId !== context.userId) {
            let [_, remark] = await UserInfo.avatarRemark(context.userId, context.currentContactUserId);
            let [avatar, nickname] = await UserInfo.avatarNickname(context.currentContactUserId);
            if (remark === '') {
                remark = nickname;
            }
            this.setState({
                remark: remark
            });
        }
        this.setState({
            avatar: userInfo.data.avatar,
            nickname: userInfo.data.nickname,
            signature: userInfo.data.signature
        });
    }

    componentDidUpdate = async (prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any) => {
        let context = this.context as Context;
        if (prevState.userId !== context.currentContactUserId) {
            await this.componentDidMount();
        }
    }

    onSignatureChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({
            signature: e.target.value
        });
    }

    onNicknameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({
            nickname: e.target.value
        });
    }

    onRemarkChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({
            remark: e.target.value
        });
    }

    onAvatarChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        let file = e.target.files![0];
        let reader = new FileReader();
        reader.onload = async (e) => {
            let avatar = e.target!.result as string;
            let resp = await HttpClient.put('/user/avatar', {}, {
                avatar: avatar
            }, true);
            if (resp.ok) {
                this.setState({
                    avatar: avatar
                });
                // context.updateAvatar(avatar);
            } else {
                console.log(resp.errMsg);
            }
        }
        reader.readAsDataURL(file);
    }

    onSignatureKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (this.state.signature === '') {
                return;
            }
            let resp = await HttpClient.put('/user/info', {}, {
                signature: this.state.signature
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-s-i')!.blur();
        }
    }

    onNicknameKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (this.state.nickname === '') {
                return;
            }
            let resp = await HttpClient.put('/user/info', {}, {
                nickname: this.state.nickname
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-n-i')!.blur();
        }
    }

    onRemarkKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (this.state.remark === '') {
                return;
            }
            let context = this.context as Context;
            let resp = await HttpClient.put('/relationship/friend', {}, {
                peer_id: Number(context.currentContactUserId),
                remark: this.state.remark
            }, true);
            if (!resp.ok) {
                console.log(resp.errMsg);
            }
            document.getElementById('c-i-r-i')!.blur();
        }
    }

    onLogout = async () => {
        let context = this.context as Context;
        await KVDB.del('access-token');
        await context.disconnect();
    }

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="contact-info">
                <div className="na"></div>
                <div className="contact-info-avatar">
                    <label htmlFor="contact-avatar">
                        <img className="avatar-img" src={this.state.avatar} alt="" />
                    </label>
                    {
                        context.currentContactUserId === context.userId &&
                        <input type="file" id='contact-avatar' hidden onChange={this.onAvatarChange} />
                    }
                </div>
                <div className="contact-info-account-id">
                    {
                        context.currentContactUserId + ""
                    }
                </div>
                <div className="contact-info-signature">
                    {
                        context.currentContactUserId !== context.userId ?
                            this.state.signature :
                            <input id='c-i-s-i' className='c-i-input' type="text" value={this.state.signature}
                                placeholder='Say something to make a different self!'
                                onChange={this.onSignatureChange} onKeyDown={this.onSignatureKeyDown} autoCorrect='off' />
                    }
                </div>
                <div className="contact-info-nickname">
                    {
                        context.currentContactUserId === context.userId
                            ?
                            <input id='c-i-n-i' className='c-i-input' type="text"
                                value={this.state.nickname} placeholder='nickname'
                                onChange={this.onNicknameChange} onKeyDown={this.onNicknameKeyDown} autoCorrect='off' />
                            :
                            this.state.nickname
                    }
                </div>
                <div className="contact-info-remark">
                    {
                        context.currentContactUserId !== context.userId &&
                        <input id='c-i-r-i' className='c-i-input' type="text"
                            value={this.state.remark} placeholder='remark'
                            onChange={this.onRemarkChange} onKeyDown={this.onRemarkKeyDown} autoCorrect='off' />
                    }
                </div>
                <div className="contact-info-logout">
                    {
                        context.currentContactUserId === context.userId &&
                        <Link to="/login">
                            <button className="logout-btn" onClick={this.onLogout}>Logout</button>
                        </Link>
                    }
                </div>
                <div className="na-0"></div>
            </div>
        )
    }
}

export default ContactInfo;