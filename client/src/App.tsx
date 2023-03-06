import { BrowserRouter, Route, Routes } from "react-router-dom";
import Chat from "./components/chat/Chat";
import Contacts from "./components/contacts/Contacts";
import More from "./components/more/More";
import './App.css'

function App() {
    return (
        <div id={"root"}>
            <BrowserRouter>
                <Routes>
                    <Route path="/" element={<Chat></Chat>} />
                    <Route path="/contacts" element={<Contacts></Contacts>} />
                    <Route path="/more" element={<More></More>} />
                </Routes>
            </BrowserRouter>
        </div>
    )
}

export default App;