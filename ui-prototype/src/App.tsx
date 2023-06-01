import { BrowserRouter, Route, Routes } from 'react-router-dom'
import './App.css'
import ChatMain from './components/chat/Main'
import SignMain from './components/sign/Sign'

function App() {
  return (
    <div id={'root'}>
      <BrowserRouter>
        <Routes>
          <Route path='/' element={<ChatMain></ChatMain>}></Route>
          <Route path='/sign' element={<SignMain></SignMain>}></Route>
        </Routes>
      </BrowserRouter>
    </div>
  )
}

export default App
