import React from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { KVDB } from '../../service/database';
import './ContactInfo.css'

class Props { }

class State { }

class ContactInfo extends React.Component<Props, State> {
    static contextType = GlobalContext;

    onLogout = async () => {
        let context = this.context as Context;
        await KVDB.del<string>('access-token');
        await context.disconnect();
    }

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="contact-info">
                <div className="na"></div>
                <div className="contact-info-avatar">
                    <img className="avatar-img" src={context.userAvatar} alt="" />
                </div>
                <div className="contact-info-account-id">
                    {
                        context.userId + ""
                    }
                </div>
                <div className="contact-info-nickname">
                    {
                        context.userNickname
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