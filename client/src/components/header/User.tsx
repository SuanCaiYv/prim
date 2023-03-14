import React from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import './User.css'

class Props { }

class State { }

class User extends React.Component<Props, State> {
    static contextType = GlobalContext;

    render(): React.ReactNode {
        let context = this.context as Context;
        return (
            <div className="user">
                <Link to="/contacts">
                    <img className="user-info-avatar" src={context.userAvatar} alt="" />
                </Link>
            </div>
        )
    }
}

export default User;