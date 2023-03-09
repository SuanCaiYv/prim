import React, { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import './Contacts.css'

class Props {
    clicked: string = '';
    onClick: any;
}

class State {
    icon: string = '/src/assets/contacts.png';
}

class Contacts extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    onClick = () => {
        this.props.onClick('contacts');
    }

    componentDidUpdate(prevProps: Readonly<Props>, prevState: Readonly<State>, snapshot?: any): void {
        if (prevProps.clicked !== this.props.clicked) {
            if (this.props.clicked === 'contacts') {
                this.setState({ icon: '/src/assets/selected.png' });
            } else {
                this.setState({ icon: '/src/assets/contacts.png' });
            }
        }
    }

    render(): ReactNode {
        return (
            <div className="contacts">
                <Link to='/contacts'>
                    <img src={this.state.icon} alt="" onClick={this.onClick} />
                </Link>
            </div>
        )
    }
}

export default Contacts;