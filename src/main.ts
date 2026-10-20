import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { store, type StyleName, type ThemeChoice } from './lib/store.svelte';

// Viste dimostrative per prove e schermate, come le tavole del prototipo:
// ?stile=nuovo&view=composizione&tema=scuro&avvio=0
// Nell'app installata l'indirizzo non ha parametri, quindi non succede nulla.
const params = new URLSearchParams(location.search);
if (params.size) {
  store.applyDemo({
    stile: (params.get('stile') as StyleName) || undefined,
    view: params.get('view') || undefined,
    tema: (params.get('tema') as ThemeChoice) || undefined,
    avvio: params.has('avvio') ? params.get('avvio') === '1' : undefined,
    offline: params.has('offline') ? params.get('offline') === '1' : undefined,
  });
}

const app = mount(App, { target: document.getElementById('app')! });

export default app;
