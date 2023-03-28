import './Dialog.css'
import React from "react"
import { createPortal } from "react-dom"
import Dialog from "./Dialog"

class Props {
    content: any
    done: () => Promise<void> = async () => { }
    trigger: boolean = false
}

class State {
    dialogVisible: boolean = false
    content: string = ''
}

class Portal extends React.Component<Props, State> {
    div: HTMLDivElement

    constructor(props: any) {
        super(props)
        this.state = new State()
        this.div = document.createElement('div')
        this.div.setAttribute('id', 'portal')
        document.body.appendChild(this.div)
    }

    componentWillUnmount(): void {
        document.body.removeChild(this.div)
    }

    done = async () => {
        await this.props.done()
        this.setState({ dialogVisible: false })
    }

    componentDidUpdate(prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any): void {
        if (this.props.trigger !== prevProps.trigger) {
            this.setState({ content: this.props.content })
            this.setState({ dialogVisible: true })
        }
    }

    render = (): React.ReactNode => {
        return (
            <div className='portal'>
                {
                    createPortal(<Dialog visible={this.state.dialogVisible} content={this.state.content} done={this.done}></Dialog>, this.div)
                }
            </div>
        )
    }
}

export default Portal
