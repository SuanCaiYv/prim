import React, { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import './Chat.css'

class Props {
    clicked: string = '';
    onClick: (type: string) => void = (_type: string) => { };
}

class State {
    icon: string = '/assets/chat.png';
}

class Chat extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = () => {
        this.props.onClick('chat');
    }

    componentDidMount(): void {
        if (this.props.clicked === 'chat') {
            this.setState({ icon: '/assets/selected.png' })
        }
    }

    componentDidUpdate(prevProps: Readonly<Props>, _prevState: Readonly<State>, _snapshot?: any): void {
        if (prevProps.clicked !== this.props.clicked) {
            if (this.props.clicked === 'chat') {
                this.setState({ icon: '/assets/selected.png' });
            } else {
                this.setState({ icon: '/assets/chat.png' });
            }
        }
    }

    render(): ReactNode {
        return (
            <div className={'chat-h'} data-tauri-drag-region>
                <Link className={'h-full w-full p-0 m-0 border-0'} to='/'>
                    <img src={this.state.icon} alt="" onClick={this.onClick} />
                </Link>
            </div>
        )
    }
}

export default Chat;