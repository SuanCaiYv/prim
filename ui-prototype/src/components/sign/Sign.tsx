import { useState } from "react"
import { useNavigate } from "react-router-dom"
import './Sign.css'

export default function SignMain() {
    let [userId, setUserId] = useState("")
    let [credential, setCredential] = useState("")
    let [avatar] = useState("")
    let navigate = useNavigate()

    const onUserIdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setUserId(e.target.value)
    }

    const onCredentialChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setCredential(e.target.value)
    }

    const onLogin = () => {
        if (userId.length === 0) {
            alert("AccountID is empty")
            return
        }
        if (credential.length === 0) {
            alert("Credential is empty")
            return
        }
        navigate('/')
    }

    return (
        <div className={'login'}>
            <div className={'login-avatar'}>
                <img src={avatar} alt="" />
            </div>
            <div className={'login-user-id'}>
                <input type="text" placeholder="AccountID" value={
                    userId.length === 0 ? "" : userId + ""
                } onChange={onUserIdChange} />
            </div>
            <div className={'login-credential'}>
                <input type="password" placeholder="Credential" value={
                    userId.length === 0 ? "" : credential
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
}