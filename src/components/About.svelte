<script lang="ts">
  // Contenuto della pagina "Informazioni su Better-Look", condiviso tra il menu
  // File (vista classica) e le Impostazioni (vista nuova).
  import Icon from '../lib/Icon.svelte';
  import Wordmark from './Wordmark.svelte';
  import { appVersion, REPO_URL } from '../lib/platform';
  import { store } from '../lib/store.svelte';

  let version = $state('');
  $effect(() => {
    appVersion().then((v) => (version = v));
  });
</script>

<div class="about">
  <Wordmark height={32} />
  <p class="ver">{version ? `Versione ${version} (anteprima)` : 'Versione di anteprima'}</p>
  <p class="lead">Better-Look è un programma di posta gratuito, libero e open source, per chi arriva da Outlook. La posta viaggia direttamente tra il tuo computer e il server del tuo provider.</p>
  <div class="links">
    <button class="btn obtn" onclick={() => store.openLink(REPO_URL)}><Icon name="code" size={17} style="color:var(--brand-ink)" />Codice sorgente</button>
    <button class="btn obtn" onclick={() => store.openLink(`${REPO_URL}#licenza`)}><Icon name="file-text" size={17} style="color:var(--brand-ink)" />Licenza</button>
    <button class="btn obtn" onclick={() => store.openLink(`${REPO_URL}/issues`)}><Icon name="bug" size={17} style="color:var(--danger)" />Segnala un problema</button>
  </div>
</div>

<style>
  .about { max-width: 640px; }
  .ver { margin: 14px 0 0; font-size: 13.5px; color: var(--ink-3); }
  .lead { margin: 26px 0 0; max-width: 60ch; font-size: 15px; line-height: 1.55; }
  .links { margin-top: 26px; display: flex; flex-wrap: wrap; gap: 8px; }
</style>
