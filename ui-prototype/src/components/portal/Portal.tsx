import { createRoot } from 'react-dom/client';
import './Portal.css'

const MessageMin = (props: {message: string}) => {
    const onMask = () => {
        document.getElementById('portal')?.remove();
    }

    return (
        <div>
            <div className={'portal-message-min'}>
                {props.message}
            </div>
            <div className={'portal-mask'} onClick={onMask}></div>
        </div>
    )
}

const alertMin = (message: string) => {
    let node = document.createElement('div')
    node.setAttribute('id', 'portal')
    document.getElementById('app')?.appendChild(node)
    let component = <MessageMin message={message} />
    createRoot(node).render(component)
}

const alertNormal = (message: string) => {}

const alertMax = (message: string) => {}

const alertComponentMin = (component: any) => {}

const alertComponentNormal = (component: any) => {}

const alertComponentMax = (component: any) => {}

const alertInteractiveMin = (component: any, onOk: () => Promise<void>, onCancel: () => Promise<void>) => {
}

export {alertMin}