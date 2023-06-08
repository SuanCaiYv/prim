import React, { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import './More.css'

class Props {
    clicked: string = '';
    onClick: any;
}

class State {
    icon: string = '/assets/more.png';
}

class More extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = () => {
        this.props.onClick('more');
    }

    componentDidUpdate(prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any): void {
        if (prevProps.clicked !== this.props.clicked) {
            if (this.props.clicked === 'more') {
                this.setState({ icon: '/assets/selected.png' });
            } else {
                this.setState({ icon: '/assets/more.png' });
            }
        }
    }

    render(): ReactNode {
        return (
            <div className={'more-h'} data-tauri-drag-region>
                <Link to='/more'>
                    <img src={this.state.icon} alt="" onClick={this.onClick} />
                </Link>
            </div>
        )
    }
}

export default More;