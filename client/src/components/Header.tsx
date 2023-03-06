import Search from './header/Search';
import Add from './header/Add';
import Chat from './header/Chat';
import Contacts from './header/Contacts';
import More from './header/More';
import './Header.css'

function Header(props: any) {
    return (
        <div className="header">
            <Search></Search>
            <Add></Add>
            <Chat></Chat>
            <Contacts></Contacts>
            <More></More>
        </div>
    )
}

export default Header;