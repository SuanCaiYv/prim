import React from "react"
import { Link } from "react-router-dom";
import { Context, GlobalContext } from "../../context/GlobalContext";
import { HttpClient } from "../../net/http";
import { KVDB } from "../../service/database";
import "./Login.css"

class Props { }

class State {
    avatar: string = ""
    userId: string = ""
    credential: string = ""
}

class Login extends React.Component<Props, State> {
    static contextType = GlobalContext;

    chatARef: React.RefObject<any>;
    constructor(props: any) {
        super(props);
        this.state = new State();
        this.chatARef = React.createRef();
    }

    chatARefClick = () => {
        this.chatARef.current.click();
    }

    onUserIdChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({
            userId: event.target.value
        })
    }

    onCredentialChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        this.setState({
            credential: event.target.value
        })
    }

    onLogin = async () => {
        let userId = this.state.userId;
        let credential = this.state.credential;
        if (userId.length === 0 || credential.length === 0) {
            return;
        }
        let resp = await HttpClient.put("/user", {}, {
            account_id: BigInt(userId) + "",
            credential: credential
        }, false)
        if (!resp.ok) {
            console.log("login failed");
            return;
        }
        await KVDB.set<bigint>("user-id", userId);
        await KVDB.set<string>("access-token", resp.data as string);
        this.chatARefClick();
    }

    componentDidMount = async (): Promise<void> => {
        let avatar = await KVDB.get<string>("avatar");
        if (avatar === undefined) {
            avatar = "/src/assets/avatar/default-avatar-1.png"
        }
        this.setState({
            avatar: avatar,
        })
        let userId = await KVDB.get<bigint>("user-id");
        if (userId === undefined) {
            userId = 0n;
            this.setState({
                userId: ""
            })
        } else {
            this.setState({
                userId: userId + ""
            })
        }
        let token = await KVDB.get<string>("access-token");
        if (token !== undefined) {
            this.setState({
                credential: "********"
            });
        }
        let resp = await HttpClient.put('/user', {}, {}, true);
        if (resp.ok) {
            this.chatARefClick();
        } else {
            console.log(resp.errMsg);
        }
    }

    render(): React.ReactNode {
        return (
            <div className="login">
                <div className="login-avatar">
                    <img className="login-avatar-img" src={this.state.avatar} alt="" />
                </div>
                <div className="login-user-id">
                    <input className="login-input" type="text" placeholder="AccountID" value={
                        this.state.userId.length === 0 ? "" : this.state.userId
                    } onChange={this.onUserIdChange}/>
                </div>
                <div className="login-credential">
                    <input className="login-input" type="password" placeholder="Credential" value={
                        this.state.userId.length === 0 ? "" : this.state.credential
                    } onChange={this.onCredentialChange}/>
                </div>
                <div className="login-a">
                    <a className="login-a-a" href="">New Here?</a>OR
                    <a className="login-a-a" href="">Forgot Credential</a>
                </div>
                <div className="login-button">
                    <button className="login-button-button" onClick={this.onLogin}>Log in</button>
                </div>
                <Link className="chat-a-direct" to="/" ref={this.chatARef}></Link>
            </div>
        )
    }
}

export default Login