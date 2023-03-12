import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Contacts.css';
import ContactInfo from './ContactInfo';

function Contacts() {
    return (
        <div className="contacts">
            <Layout>
                <Header clicked={"contacts"}></Header>
                <List></List>
                <Main>
                    <ContactInfo></ContactInfo>
                </Main>
            </Layout>
        </div>
    )
}

export default Contacts;