import { createRoot } from 'react-dom/client';
import './Portal.css'

const MessageMin = (props: {message: string}) => {
    const onMask = () => {
        document.getElementById('portal1')?.remove();
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
        document.getElementById('portal2')?.remove();
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

const OperationResult = (props: {succeed: boolean}) => {
    const onMask = () => {
        document.getElementById('portal3')?.remove();
    }

    return (
        <div>
            <div className={'portal-operation-result'}>
                {props.succeed ? 'Succeed' : 'Failed!'}
            </div>
            <div className={'portal-mask'} onClick={onMask}></div>
        </div>
    )
}

const alertMin = (message: string) => {
    let node = document.createElement('div')
    node.setAttribute('id', 'portal1')
    document.getElementById('app')?.appendChild(node)
    let component = <MessageMin message={message} />
    createRoot(node).render(component)
}

// @ts-ignore
const alertNormal = (message: string) => {}

// @ts-ignore
const alertMax = (message: string) => {}

// @ts-ignore
const alertComponentMin = (component: any) => {}

const alertComponentNormal = (cmt: any): Function => {
    let node = document.createElement('div')
    node.setAttribute('id', 'portal2')
    document.getElementById('app')?.appendChild(node)
    let component = <ComponentNormal component={cmt} />
    createRoot(node).render(component)
    const onMask = () => {
        document.getElementById('portal2')?.remove();
    }
    return onMask;
}

// @ts-ignore
const alertComponentMax = (component: any) => {}

// @ts-ignore
const alertInteractiveMin = (component: any, onOk: () => Promise<void>, onCancel: () => Promise<void>) => {
}

const operationResult = (succeed: boolean) => {
    let node = document.createElement('div')
    node.setAttribute('id', 'portal3')
    document.getElementById('app')?.appendChild(node)
    let component = <OperationResult succeed={succeed} />
    createRoot(node).render(component)
    const onMask = () => {
        document.getElementById('portal3')?.remove();
    }
    setTimeout(onMask, 1000)
    return onMask;
}

export {alertMin, alertComponentNormal, operationResult}