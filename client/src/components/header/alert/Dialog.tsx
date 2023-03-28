import React from 'react'
import './Dialog.css'

class Props {
    content: any
    done: () => Promise<void> = async () => { }
    visible: boolean = false
}

class State { }

export default class Dialog extends React.Component<Props, State> {
    constructor(props: any) {
        super(props)
    }

    render() {
        return (
            this.props.visible && (
                <div className='container'>
                    <div className='content'>{this.props.content}</div>
                    <div className='btn' onClick={this.props.done}>Done</div>
                </div>
            )
        )
    }
}
