import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import ComposeApp from './components/ComposeApp.svelte';
import { store, type StyleName, type ThemeChoice } from './lib/store.svelte';

const params = new URLSearchParams(location.search);
const target = document.getElementById('app')!;

if (params.get('finestra') === 'messaggio') {
  // Finestra separata per scrivere un messaggio.
  store.mode = 'messaggio';
  store.onboarding = false;
  mount(ComposeApp, { target });
} else {
  mount(App, { target });
  void store.init().then(() => {
    // Viste dimostrative per prove e schermate, come le tavole del prototipo:
    // ?stile=nuovo&view=composizione&tema=scuro&avvio=0
    // Nell'app installata l'indirizzo non ha parametri, quindi non succede nulla.
    if (params.size) {
      return store.applyDemo({
        stile: (params.get('stile') as StyleName) || undefined,
        view: params.get('view') || undefined,
        tema: (params.get('tema') as ThemeChoice) || undefined,
        accento: params.get('accento') || undefined,
        avvio: params.has('avvio') ? params.get('avvio') === '1' : undefined,
        offline: params.has('offline') ? params.get('offline') === '1' : undefined,
      });
    }
  });
}
