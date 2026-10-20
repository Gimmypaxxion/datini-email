<script lang="ts">
  // Contenuto della pagina "Informazioni su Datini", condiviso tra il menu
  // File (vista classica) e le Impostazioni (vista nuova).
  import Icon from '../lib/Icon.svelte';
  import Wordmark from './Wordmark.svelte';
  import { appVersion } from '../lib/platform';

  let version = $state('');
  $effect(() => {
    appVersion().then((v) => (version = v));
  });
</script>

<div class="about">
  <Wordmark height={32} />
  <p class="ver">{version ? `Versione ${version} (anteprima)` : 'Versione di anteprima'}</p>
  <p class="lead">Datini è un programma di posta libero e open source. La posta viaggia direttamente tra il tuo computer e il server del tuo provider.</p>
  <p class="story">Porta il nome di Francesco Datini, mercante di Prato del Trecento. Il suo archivio conserva circa 143.000 lettere d’affari scambiate con quasi trecento città.</p>
  <div class="links">
    <button class="btn obtn"><Icon name="code" size={17} style="color:var(--brand-ink)" />Codice sorgente</button>
    <button class="btn obtn"><Icon name="file-text" size={17} style="color:var(--brand-ink)" />Licenza</button>
    <button class="btn obtn"><Icon name="bug" size={17} style="color:var(--danger)" />Segnala un problema</button>
  </div>
</div>

<style>
  .about { max-width: 640px; }
  .ver { margin: 14px 0 0; font-size: 13.5px; color: var(--ink-3); }
  .lead { margin: 26px 0 0; max-width: 60ch; font-size: 15px; line-height: 1.55; }
  .story { margin: 12px 0 0; max-width: 60ch; font-size: 15px; line-height: 1.55; color: var(--ink-2); }
  .links { margin-top: 26px; display: flex; flex-wrap: wrap; gap: 8px; }
</style>
