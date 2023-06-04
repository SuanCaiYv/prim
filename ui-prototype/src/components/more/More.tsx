import React from 'react';
import Header from '../Header';
import Layout from '../Layout';
import List from '../List';
import Main from '../Main';
import './More.css';

class More extends React.Component {

    componentDidMount(): void {
    }

    render(): React.ReactNode {
        return (
            <div className="more">
                <Layout>
                    <Header clicked={"more"}></Header>
                    <List>
                    </List>
                    <Main></Main>
                </Layout>
            </div>
        )
    }
}

export default More;