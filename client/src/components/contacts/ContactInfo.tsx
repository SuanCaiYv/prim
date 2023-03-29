import React from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { KVDB } from '../../service/database';
import { UserInfo } from '../../service/user/userInfo';
import './ContactInfo.css'

class Props { }

class State {
    avatar: string = "";
    nickname: string = "";
}

class ContactInfo extends React.Component<Props, State> {
    static contextType = GlobalContext;

    constructor(props: Props) {
        super(props);
        this.state = new State();
    }

    async componentDidMount() {
        let context = this.context as Context;
        let [avatar, nickname] = await UserInfo.avatarNickname(context.userId);
        this.setState({
            avatar: avatar,
            nickname: nickname
        });
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
                    <img className="avatar-img" src={this.state.avatar} alt="" />
                </div>
                <div className="contact-info-account-id">
                    {
                        context.userId + ""
                    }
                </div>
                <div className="contact-info-nickname">
                    {
                        this.state.nickname
                    }
                </div>
                <div className="contact-info-logout">
                    <Link to="/login">
                        <button className="logout-btn" onClick={this.onLogout}>Logout</button>
                    </Link>
                </div>
                <div className="na-0"></div>
            </div>
        )
    }
}

export default ContactInfo;