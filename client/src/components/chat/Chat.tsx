import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Chat.css';
import UserMsgListItem from './UserMsgListItem';

function Chat() {
    return (
        <div className="chat">
            <Layout>
                <Header clicked={"chat"}></Header>
                <List>
                    <UserMsgListItem msg={'aaa'}></UserMsgListItem>
                </List>
                <Main></Main>
            </Layout>
        </div>
    )
}

export default Chat;