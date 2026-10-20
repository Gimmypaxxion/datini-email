<script lang="ts">
  // Aggiungi account: indirizzo e password bastano per i provider più diffusi.
  // Se il motore non trova da solo i server, si aprono le impostazioni
  // manuali già precompilate con i nomi più probabili.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import type { AccountKind, Security, ServerConfig } from '../lib/types';

  type Step = 'form' | 'cerco' | 'fatto';

  let step = $state<Step>('form');
  let email = $state('');
  let password = $state('');
  let name = $state('');
  let kind = $state<AccountKind>('imap');
  let manual = $state(false);
  let showPw = $state(false);
  let error = $state('');
  let errorField = $state<'email' | 'password' | ''>('');
  let added = $state('');

  let inHost = $state('');
  let inPort = $state(993);
  let inSec = $state<Security>('ssl');
  let outHost = $state('');
  let outPort = $state(465);
  let outSec = $state<Security>('ssl');
  let user = $state('');

  let emailEl: HTMLInputElement | undefined = $state();

  const domain = $derived(email.includes('@') ? email.split('@')[1].trim().toLowerCase() : '');

  $effect(() => {
    if (store.wizard.open) {
      step = 'form';
      error = '';
      errorField = '';
      setTimeout(() => emailEl?.focus(), 0);
    }
  });

  function fillManual() {
    const d = domain || 'azienda.it';
    inHost ||= `${kind === 'imap' ? 'imap' : 'pop'}.${d}`;
    inPort = kind === 'imap' ? (inSec === 'ssl' ? 993 : 143) : inSec === 'ssl' ? 995 : 110;
    outHost ||= `smtp.${d}`;
    user ||= email.trim();
  }

  function setKind(k: AccountKind) {
    kind = k;
    if (manual) {
      inHost = inHost.replace(/^(imap|pop|pop3)\./, `${k === 'imap' ? 'imap' : 'pop'}.`);
      fillManual();
    }
  }

  function toggleManual() {
    manual = !manual;
    if (manual) fillManual();
  }

  function close() {
    if (step === 'cerco') return;
    store.closeWizard();
    if (step === 'fatto') {
      email = '';
      password = '';
      name = '';
      manual = false;
      inHost = outHost = user = '';
    }
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    errorField = '';
    if (!email.trim()) {
      error = 'Scrivi il tuo indirizzo email.';
      errorField = 'email';
      return;
    }
    if (!password) {
      error = 'Scrivi la password della casella.';
      errorField = 'password';
      return;
    }
    step = 'cerco';
    const server = (host: string, port: number, security: Security): ServerConfig => ({ host: host.trim(), port: Number(port), security, username: user.trim() || email.trim() });
    const r = await store.addAccount({
      name: name.trim(),
      email: email.trim(),
      password,
      kind,
      ...(manual && inHost.trim() && outHost.trim() ? { incoming: server(inHost, inPort, inSec), outgoing: server(outHost, outPort, outSec) } : {}),
    });
    if (r.ok) {
      added = r.account.email;
      password = '';
      step = 'fatto';
      return;
    }
    step = 'form';
    error = r.message;
    if (r.kind === 'credenziali') errorField = 'password';
    if (r.kind === 'manuale' && !manual) {
      manual = true;
      fillManual();
    }
  }
</script>

{#if store.wizard.open}
  <div class="scrim fade-in">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="dlg rise"
      role="dialog"
      aria-modal="true"
      aria-labelledby="wiz-title"
      tabindex="-1"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          close();
        }
      }}
    >
      {#if step === 'fatto'}
        <div class="done">
          <Icon name="circle-check" size={48} stroke={1.4} style="color:var(--green)" />
          <h2 id="wiz-title">Account aggiunto</h2>
          <p>Datini sta scaricando la posta di <strong>{added}</strong>. I messaggi compaiono man mano, a partire dai più recenti.</p>
          <button class="primary-btn" onclick={close}>Fine</button>
        </div>
      {:else}
        <header>
          <h2 id="wiz-title">{store.wizard.first ? 'Aggiungi il tuo account di posta' : 'Aggiungi un account'}</h2>
          <button class="ibtn btn" onclick={close} disabled={step === 'cerco'} aria-label="Chiudi" title="Chiudi (Esc)"><Icon name="x" /></button>
        </header>
        <form class="scroll body" onsubmit={submit} novalidate>
          <p class="intro">Scrivi indirizzo e password: Datini trova da solo le impostazioni dei provider più diffusi, come Aruba, Libero, Register.it e Gmail.</p>

          <fieldset disabled={step === 'cerco'}>
            <label class="tf" data-bad={errorField === 'email' ? 'si' : 'no'}>
              <span>Indirizzo email</span>
              <input bind:this={emailEl} type="email" bind:value={email} autocomplete="email" placeholder="nome@azienda.it" spellcheck="false" />
            </label>
            <label class="tf pw" data-bad={errorField === 'password' ? 'si' : 'no'}>
              <span>Password</span>
              <input type={showPw ? 'text' : 'password'} bind:value={password} autocomplete="current-password" />
              <button type="button" class="ibtn btn eye" onclick={() => (showPw = !showPw)} aria-label={showPw ? 'Nascondi password' : 'Mostra password'} title={showPw ? 'Nascondi password' : 'Mostra password'}><Icon name={showPw ? 'eye-off' : 'eye'} size={17} /></button>
            </label>
            <label class="tf">
              <span>Il tuo nome <span class="hint">come lo vedono i destinatari</span></span>
              <input type="text" bind:value={name} autocomplete="name" placeholder="Nome e cognome" />
            </label>

            <div class="kind" role="radiogroup" aria-label="Tipo di account">
              <span class="kl">Tipo di account</span>
              <div class="pills">
                <label class="pill" data-on={kind === 'imap' ? 'si' : 'no'}><input type="radio" name="kind" checked={kind === 'imap'} onchange={() => setKind('imap')} />IMAP (consigliato)</label>
                <label class="pill" data-on={kind === 'pop3' ? 'si' : 'no'}><input type="radio" name="kind" checked={kind === 'pop3'} onchange={() => setKind('pop3')} />POP</label>
              </div>
              <p class="hint">{kind === 'imap' ? 'La posta resta sul server ed è uguale su tutti i tuoi dispositivi.' : 'La posta viene scaricata su questo computer, che ne tiene l’unica copia completa.'}</p>
            </div>

            <button type="button" class="lbtn disc" aria-expanded={manual} onclick={toggleManual}>
              <Icon name={manual ? 'chevron-down' : 'chevron-right'} size={15} stroke={1.8} />Impostazioni manuali
            </button>
            {#if manual}
              <div class="manual">
                <div class="srvrow">
                  <label class="tf grow"><span>Server posta in arrivo ({kind === 'imap' ? 'IMAP' : 'POP'})</span><input bind:value={inHost} spellcheck="false" /></label>
                  <label class="tf port"><span>Porta</span><input type="number" bind:value={inPort} min="1" max="65535" /></label>
                  <label class="tf sec"><span>Sicurezza</span>
                    <select bind:value={inSec} onchange={() => (inPort = kind === 'imap' ? (inSec === 'ssl' ? 993 : 143) : inSec === 'ssl' ? 995 : 110)}>
                      <option value="ssl">SSL/TLS</option><option value="starttls">STARTTLS</option>
                    </select>
                  </label>
                </div>
                <div class="srvrow">
                  <label class="tf grow"><span>Server posta in uscita (SMTP)</span><input bind:value={outHost} spellcheck="false" /></label>
                  <label class="tf port"><span>Porta</span><input type="number" bind:value={outPort} min="1" max="65535" /></label>
                  <label class="tf sec"><span>Sicurezza</span>
                    <select bind:value={outSec} onchange={() => (outPort = outSec === 'ssl' ? 465 : 587)}>
                      <option value="ssl">SSL/TLS</option><option value="starttls">STARTTLS</option>
                    </select>
                  </label>
                </div>
                <label class="tf"><span>Nome utente <span class="hint">di solito è l’indirizzo email</span></span><input bind:value={user} spellcheck="false" /></label>
              </div>
            {/if}
          </fieldset>

          {#if error}
            <p class="error" role="alert"><Icon name="alert-circle" size={17} />{error}</p>
          {/if}

          {#if step === 'cerco'}
            <div class="busy" role="status">
              <div class="progress"></div>
              <span>{manual ? 'Verifico i server e la password' : `Cerco le impostazioni di ${domain || 'questo dominio'} e verifico la password`}</span>
            </div>
          {/if}

          <p class="privacy"><Icon name="shield-lock" size={17} style="color:var(--brand-ink)" />La password resta nel portachiavi di questo computer. La posta viaggia direttamente tra il computer e il server del tuo provider, senza passare da altri.</p>

          <div class="actions">
            <button type="button" class="sec-btn" onclick={close} disabled={step === 'cerco'}>{store.wizard.first ? 'Più tardi' : 'Annulla'}</button>
            <button type="submit" class="primary-btn" disabled={step === 'cerco'}>Connetti</button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dlg { width: min(560px, 100%); max-height: 100%; display: flex; flex-direction: column; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); overflow: hidden; outline: none; }
  header { display: flex; align-items: center; gap: 12px; padding: 20px 16px 0 28px; }
  h2 { margin: 0; flex: 1; font-size: 20px; font-weight: 600; }
  .body { overflow: auto; padding: 8px 28px 24px; }
  .intro { margin: 0 0 20px; font-size: 14px; line-height: 1.5; color: var(--ink-2); }
  fieldset { margin: 0; padding: 0; border: 0; min-width: 0; display: flex; flex-direction: column; gap: 16px; }
  .pw { position: relative; }
  .pw input { padding-right: 40px; }
  .eye { position: absolute; right: 2px; bottom: 1px; width: 32px; height: 32px; }
  .kind { display: flex; flex-direction: column; gap: 8px; }
  .kl { font-size: 13.5px; font-weight: 600; }
  .pills { display: flex; gap: 8px; flex-wrap: wrap; }
  .hint { margin: 0; font-size: 12.5px; color: var(--ink-3); font-weight: 400; }
  .disc { align-self: flex-start; height: 30px; display: inline-flex; align-items: center; gap: 4px; padding: 0 4px; font-size: 14px; font-weight: 600; }
  .manual { display: flex; flex-direction: column; gap: 14px; padding: 16px; border-radius: 6px; background: var(--rail); }
  .srvrow { display: grid; grid-template-columns: minmax(0, 1fr) 84px 118px; gap: 10px; }
  .error { margin: 18px 0 0; display: flex; gap: 8px; align-items: flex-start; font-size: 14px; line-height: 1.45; color: var(--danger); }
  .error :global(svg) { margin-top: 1px; }
  .busy { margin-top: 18px; display: flex; flex-direction: column; gap: 8px; font-size: 13.5px; color: var(--ink-2); }
  .privacy { margin: 20px 0 0; display: flex; gap: 10px; align-items: flex-start; font-size: 13px; line-height: 1.5; color: var(--ink-2); }
  .privacy :global(svg) { margin-top: 1px; }
  .actions { margin-top: 22px; display: flex; justify-content: flex-end; gap: 8px; }
  .done { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 40px 36px 32px; text-align: center; }
  .done h2 { flex: none; margin-top: 6px; }
  .done p { margin: 0 0 14px; max-width: 42ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  @media (max-width: 560px) { .srvrow { grid-template-columns: 1fr 1fr; } .srvrow .grow { grid-column: 1 / -1; } }
</style>
