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

const ComponentNormal = (props: {component: any}) => {
    const onMask = () => {
        document.getElementById('portal')?.remove();
    }

    return (
        <div>
            <div className={'portal-component-normal'}>
                {props.component}
            </div>
            <div className={'portal-mask'} onClick={onMask}></div>
        </div>
    )
};

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

const alertComponentNormal = (cmt: any) => {
    let node = document.createElement('div')
    node.setAttribute('id', 'portal')
    document.getElementById('app')?.appendChild(node)
    let component = <ComponentNormal component={cmt} />
    createRoot(node).render(component)
}

const alertComponentMax = (component: any) => {}

const alertInteractiveMin = (component: any, onOk: () => Promise<void>, onCancel: () => Promise<void>) => {
}

export {alertMin, alertComponentNormal}