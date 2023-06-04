import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Contacts.css';
import ContactInfo from './ContactInfo';
import { useEffect, useState } from 'react';
import { ContactItemData } from '../../entity/inner';
import Relationship from '../../service/user/relationship';
import ContactListItem from './ContactListItem';

const ContactsMain = () => {
    let [contacts, setContacts] = useState<Array<ContactItemData>>([]);

    useEffect(() => {
        (async () => {
            let contacts = await Relationship.contactList();
            setContacts(contacts);
        })();
        return () => {};
    });

    return (
        <div className={'contacts'}>
            <Layout>
                <Header clicked={"contacts"}></Header>
                <List>
                    {
                        contacts.map((contact: ContactItemData, index: number) => {
                            return <ContactListItem key={index} userId={contact.userId} avatar={contact.avatar} remark={contact.remark} nickname={contact.nickname}></ContactListItem>
                        })
                    }
                </List>
                <Main>
                    <ContactInfo></ContactInfo>
                </Main>
            </Layout>
        </div>
    )
}

export default ContactsMain;