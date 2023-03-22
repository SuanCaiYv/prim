import React from 'react';
import { Link } from 'react-router-dom';
import { Context, GlobalContext } from '../../context/GlobalContext';
import { UserInfo } from '../../service/user/userInfo';
import './User.css'

class Props { }

class State {
    avatar: string = "";
    nickname: string = "";
}

class User extends React.Component<Props, State> {
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

    render =(): React.ReactNode => {
        return (
            <div className="user">
                <Link to="/contacts">
                    <img className="user-info-avatar" src={this.state.avatar} alt="" />
                </Link>
            </div>
        )
    }
}

export default User;