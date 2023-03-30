import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './Contacts.css';
import ContactInfo from './ContactInfo';
import React, { ReactNode } from 'react';
import { ContactItemData } from '../../entity/inner';
import Relationship from '../../service/user/relationship';
import ContactListItem from './ContactListItem';

class Props { }

class State {
    contacts: Array<ContactItemData> = [];
}

class Contacts extends React.Component<Props, State> {
    constructor(props: any) {
        super(props);
        this.state = new State();
    }

    componentDidMount = async (): Promise<void> => {
        let contacts = await Relationship.contactList();
        console.log(contacts);
        this.setState({
            contacts: contacts
        });
    }

    render = (): React.ReactNode => {
        return (
            <div className="contacts">
                <Layout>
                    <Header clicked={"contacts"}></Header>
                    <List>
                        {
                            this.state.contacts.map((contact: ContactItemData, index: number) => {
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
}

export default Contacts;