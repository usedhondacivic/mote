import { mount } from 'svelte'
import './styles/third_party.css'
import './styles/index.css'
import App from './App.svelte'

const app = mount(App, {
    target: document.getElementById('app')!,
})

export default app
