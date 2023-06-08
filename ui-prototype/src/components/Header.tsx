import Search from './header/Search';
import Add from './header/Add';
import Chat from './header/Chat';
import Contacts from './header/Contacts';
import More from './header/More';
import './Header.css'
import React, { ReactNode } from 'react';
import User from './header/User';
import { Link } from 'react-router-dom';

class Props {
    clicked: string = '';
}

class State {
    clicked: string = 'chat';
}

class Header extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    componentDidMount(): void {
        this.setState({ clicked: this.props.clicked });
    }

    onClick = (type: string) => {
        this.setState({clicked: type});
    }

    render(): ReactNode {
        return (
            <div className="header">
                <Search></Search>
                <Add></Add>
                <Chat clicked={this.state.clicked} onClick={this.onClick}></Chat>
                <Contacts clicked={this.state.clicked} onClick={this.onClick}></Contacts>
                <More clicked={this.state.clicked} onClick={this.onClick}></More>
                <Link className={'test-btn'} to={'/t'}>Test</Link>
                <User></User>
            </div>
        )
    }
}

export default Header;