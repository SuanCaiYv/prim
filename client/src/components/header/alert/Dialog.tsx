import React from 'react'
import './Dialog.css'

class Props {
    contentList: Array<any> = []
    done: () => Promise<void> = async () => { }
    visible: boolean = false
}

class State {
    content: any
    index: number = 0
}

export default class Dialog extends React.Component<Props, State> {
    constructor(props: any) {
        super(props)
        this.state = new State()
    }

    componentDidMount = (): void => {
        this.setState({
            content: this.props.contentList[0],
        })
    }

    componentDidUpdate(prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any): void {
        if (prevProps.visible !== this.props.visible) {
            this.setState({
                content: this.props.contentList[0],
                index: 0,
            })
        }
    }

    onSwitch = (prev: boolean) => {
        let i = this.state.index
        if (prev) {
            i --;
        } else {
            i ++;
        }
        i = (this.props.contentList.length + i) % this.props.contentList.length
        this.setState({
            index: i,
            content: this.props.contentList[i],
        })
    }

    render = () => {
        return (
            this.props.visible && (
                <div className='container'>
                    <div className='content'>{this.state.content}</div>
                    <div className='btn-l' onClick={() => {this.onSwitch(true)}}>P</div>
                    <div className='btn' onClick={this.props.done}>Done</div>
                    <div className='btn-r' onClick={() => {this.onSwitch(false)}}>N</div>
                </div>
            )
        )
    }
}
