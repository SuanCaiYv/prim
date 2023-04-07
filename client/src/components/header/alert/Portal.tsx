import './Dialog.css'
import React from "react"
import { createPortal } from "react-dom"
import Dialog from "./Dialog"

class Props {
    contentList: Array<any> = []
    done: () => Promise<void> = async () => { }
    trigger: boolean = false
}

class State {
    dialogVisible: boolean = false
    contentList: Array<any> = []
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
        setTimeout(() => {this.setState({ dialogVisible: false })}, 1200)
    }

    componentDidUpdate(prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any): void {
        if (this.props.trigger !== prevProps.trigger) {
            this.setState({ contentList: this.props.contentList })
            this.setState({ dialogVisible: true })
        }
    }

    render = (): React.ReactNode => {
        return (
            <div className='portal'>
                {
                    createPortal(<Dialog visible={this.state.dialogVisible} contentList={this.state.contentList} done={this.done}></Dialog>, this.div)
                }
            </div>
        )
    }
}

export default Portal
