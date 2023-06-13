import { useContext, useState } from "react"
import { useNavigate } from "react-router-dom"
import './Sign.css'
import { Context, GlobalContext } from "../../context/GlobalContext"
import { alertMin } from "../portal/Portal"
import { HttpClient, setBaseUrl } from "../../net/http"
import { KVDB } from "../../service/database"
import { UserInfo } from "../../service/user/userInfo"

export default function SignMain() {
    let [userId, setUserId] = useState("")
    let [credential, setCredential] = useState("")
    let [secretPage, setSecretPage] = useState(false);
    let navigate = useNavigate()
    let context = useContext(GlobalContext) as Context;

    const onUserIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setUserId(e.target.value)
    }

    const onCredentialChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setCredential(e.target.value)
    }

    const onLogin = async () => {
        let resp = await HttpClient.put('/user', {}, {}, true);
        if (resp.ok) {
            navigate('/');
            return;
        }
        if (userId.length === 0) {
            alertMin("AccountID is empty")
            return
        }
        if (credential.length === 0) {
            alertMin("Credential is empty")
            return
        }
        resp = await HttpClient.put("/user", {}, {
            account_id: Number(userId),
            credential: credential
        }, false)
        if (!resp.ok) {
            alertMin('AccountID or Credential is wrong')
            return;
        }
        await KVDB.set("user-id", BigInt(userId));
        await UserInfo.avatarNickname(BigInt(userId));
        await KVDB.set("access-token", resp.data as string);
        try { await context.setup() } catch (e) {
            console.log(e);
            return;
        }
        navigate('/')
    }

    const handleKeyDown = async (e: React.KeyboardEvent<HTMLDivElement>) => {
        if (e.ctrlKey && e.key === 'm') {
            setSecretPage(!secretPage);
            return;
        }
        if (e.key === 'Enter') {
            if (!secretPage) {
                await onLogin();
            } else {
                setBaseUrl(credential);
            }
        }
    }

    return (
        secretPage ? (
            <div className={'sign'} data-tauri-drag-region onKeyDown={handleKeyDown}>
                <div className={'login-avatar'}>
                    <img className={'bg-white'} src={'/assets/secret.png'} alt=""/>
                </div>
                <div className={'login-user-id'}>
                </div>
                <div className={'login-credential'}>
                    <input type="text" placeholder="SecretAddress" value={
                        credential
                    } onChange={onCredentialChange} />
                </div>
                <div className={'login-a'}>
                    <span className={'text-black text-lg font-semibold'}>Type In Your <strong>Secret</strong> Backend Address</span>
                </div>
                <div className="login-button">
                    <button onClick={onLogin} className={'bg-gradient-to-r from-pink-300 from-10% via-indigo-300 via-30% to-sky-500 to-70%'}>Check In</button>
                </div>
            </div>
        ) : (
            <div className={'sign'} data-tauri-drag-region onKeyDown={handleKeyDown}>
                <div className={'login-avatar'}>
                    <img src={'/assets/icon.png'} alt="" />
                </div>
                <div className={'login-user-id'}>
                    <input type="text" placeholder="AccountID" value={
                        userId.length === 0 ? "" : userId + ""
                    } onChange={onUserIdChange} />
                </div>
                <div className={'login-credential'}>
                    <input type="password" placeholder="Credential" value={
                        credential
                    } onChange={onCredentialChange} />
                </div>
                <div className={'login-a'}>
                    <a href="">New Here?</a>OR
                    <a href="">Forgot Credential</a>
                </div>
                <div className="login-button">
                    <button onClick={onLogin}>Log in</button>
                </div>
            </div>
        )
    )
}