<script lang="ts">
  // Primo avvio: "Da quale Outlook arrivi?". La scelta imposta il preset;
  // tutto si può cambiare dopo da Impostazioni › Aspetto.
  import Icon from '../lib/Icon.svelte';
  import Wordmark from './Wordmark.svelte';
  import { store, type StyleName } from '../lib/store.svelte';

  const cards: { id: StyleName; title: string; lines: string[] }[] = [
    { id: 'classico', title: 'Outlook classico', lines: ['Barra dei comandi estesa', 'Menu File', 'Nuovi messaggi in una finestra separata'] },
    { id: 'nuovo', title: 'Nuovo Outlook', lines: ['Barra dei comandi compatta', 'Impostazioni con l’ingranaggio', 'Nuovi messaggi nel riquadro di lettura'] },
  ];
</script>

{#if store.onboarding}
  <div class="scrim" style="z-index:40">
    <div class="dlg rise" role="dialog" aria-modal="true" aria-labelledby="ob-title">
      <Wordmark height={24} />
      <h1 id="ob-title">Da quale Outlook arrivi?</h1>
      <p class="intro">Mettiamo i comandi dove sei abituato a trovarli. Puoi cambiare ogni cosa quando vuoi dalle impostazioni.</p>
      <div class="two cards" role="radiogroup" aria-labelledby="ob-title">
        {#each cards as card (card.id)}
          {@const on = store.obChoice === card.id}
          <label class="obc" data-on={on ? 'si' : 'no'}>
            <input type="radio" name="ob" checked={on} onchange={() => (store.obChoice = card.id)} />
            <span class="pic" aria-hidden="true">
              {#if card.id === 'classico'}
                <svg viewBox="0 0 240 136">
                  <rect x="4" y="4" width="232" height="128" rx="5" style="fill:var(--ground);stroke:var(--rule-2)" />
                  <path d="M4.5 9.5a5 5 0 0 1 5 -5h221a5 5 0 0 1 5 5v6.5h-231z" style="fill:var(--brand)" />
                  <rect x="88" y="7" width="64" height="6" rx="1.5" style="fill:#FFFFFF;opacity:.9" />
                  <rect x="12" y="20" width="11" height="3" rx="1" style="fill:var(--brand)" />
                  <rect x="27" y="20" width="14" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="45" y="20" width="14" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="10" y="27" width="220" height="30" rx="3" style="fill:var(--sheet)" />
                  <rect x="15" y="31" width="15" height="15" rx="2" style="fill:none;stroke:var(--brand);stroke-width:1.6" />
                  <rect x="15" y="50" width="15" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="37" y="31" width="1" height="22" style="fill:var(--rule-2)" />
                  <rect x="43" y="32" width="11" height="11" rx="2" style="fill:var(--brand);opacity:.6" />
                  <rect x="43" y="47" width="11" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="59" y="32" width="11" height="11" rx="2" style="fill:var(--green);opacity:.6" />
                  <rect x="59" y="47" width="11" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="76" y="31" width="1" height="22" style="fill:var(--rule-2)" />
                  <rect x="82" y="32" width="11" height="11" rx="2" style="fill:var(--brand);opacity:.6" />
                  <rect x="82" y="47" width="11" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="98" y="32" width="11" height="11" rx="2" style="fill:var(--brand);opacity:.6" />
                  <rect x="98" y="47" width="11" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="114" y="32" width="11" height="11" rx="2" style="fill:var(--amber);opacity:.6" />
                  <rect x="114" y="47" width="11" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="131" y="31" width="1" height="22" style="fill:var(--rule-2)" />
                  <rect x="137" y="33" width="28" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="137" y="40" width="24" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="137" y="47" width="30" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="10" y="62" width="40" height="64" rx="3" style="fill:var(--ground)" />
                  <rect x="14" y="68" width="30" height="5" rx="1" style="fill:var(--brand-soft)" />
                  <rect x="54" y="62" width="62" height="64" rx="3" style="fill:var(--sheet)" />
                  <rect x="56" y="67" width="2" height="9" rx="1" style="fill:var(--brand)" />
                  <rect x="61" y="68" width="48" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="82" width="42" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="96" width="48" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="110" width="38" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="120" y="62" width="110" height="64" rx="3" style="fill:var(--sheet)" />
                  <rect x="143" y="79" width="92" height="54" rx="4" style="fill:#000000;opacity:.1" />
                  <rect x="140" y="76" width="92" height="54" rx="4" style="fill:var(--sheet);stroke:var(--rule-2)" />
                  <path d="M140.5 80a3.5 3.5 0 0 1 3.5 -3.5h84a3.5 3.5 0 0 1 3.5 3.5v4h-91z" style="fill:var(--brand)" />
                  <rect x="146" y="90" width="13" height="22" rx="2" style="fill:var(--brand)" />
                  <rect x="164" y="91" width="60" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="164" y="99" width="60" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="164" y="107" width="44" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="146" y="118" width="78" height="2" rx="1" style="fill:var(--rule-2)" />
                </svg>
              {:else}
                <svg viewBox="0 0 240 136">
                  <rect x="4" y="4" width="232" height="128" rx="5" style="fill:var(--ground);stroke:var(--rule-2)" />
                  <path d="M4.5 9.5a5 5 0 0 1 5 -5h221a5 5 0 0 1 5 5v6.5h-231z" style="fill:var(--brand)" />
                  <rect x="88" y="7" width="64" height="6" rx="1.5" style="fill:#FFFFFF;opacity:.9" />
                  <circle cx="224" cy="10" r="2.6" style="fill:none;stroke:#FFFFFF;stroke-width:1.2" />
                  <rect x="12" y="20" width="14" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="30" y="20" width="14" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="48" y="20" width="14" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="10" y="27" width="220" height="11" rx="3" style="fill:var(--sheet)" />
                  <rect x="13" y="29.5" width="26" height="6" rx="1.5" style="fill:var(--brand)" />
                  <rect x="45" y="30" width="5" height="5" rx="1" style="fill:var(--ink-3);opacity:.5" />
                  <rect x="54" y="30" width="5" height="5" rx="1" style="fill:var(--green);opacity:.7" />
                  <rect x="63" y="30" width="5" height="5" rx="1" style="fill:var(--danger);opacity:.6" />
                  <rect x="75" y="30" width="5" height="5" rx="1" style="fill:var(--brand);opacity:.7" />
                  <rect x="84" y="30" width="5" height="5" rx="1" style="fill:var(--brand);opacity:.7" />
                  <rect x="93" y="30" width="5" height="5" rx="1" style="fill:var(--brand);opacity:.7" />
                  <rect x="105" y="30" width="5" height="5" rx="1" style="fill:var(--ink-3);opacity:.5" />
                  <rect x="114" y="30" width="5" height="5" rx="1" style="fill:var(--danger);opacity:.6" />
                  <rect x="10" y="43" width="40" height="83" rx="3" style="fill:var(--ground)" />
                  <rect x="14" y="48" width="30" height="5" rx="1" style="fill:var(--brand-soft)" />
                  <rect x="54" y="43" width="62" height="83" rx="3" style="fill:var(--sheet)" />
                  <rect x="56" y="48" width="2" height="9" rx="1" style="fill:var(--brand)" />
                  <rect x="61" y="49" width="48" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="63" width="42" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="77" width="48" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="91" width="38" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="61" y="105" width="44" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="120" y="43" width="110" height="83" rx="3" style="fill:var(--sheet)" />
                  <rect x="126" y="49" width="22" height="7" rx="1.5" style="fill:var(--brand)" />
                  <rect x="126" y="63" width="96" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="126" y="71" width="96" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="126" y="79" width="70" height="3" rx="1" style="fill:var(--rule-2)" />
                  <rect x="126" y="91" width="90" height="2" rx="1" style="fill:var(--rule-2)" />
                  <rect x="126" y="97" width="80" height="2" rx="1" style="fill:var(--rule-2)" />
                  <rect x="126" y="103" width="60" height="2" rx="1" style="fill:var(--rule-2)" />
                </svg>
              {/if}
            </span>
            <span class="head"><span class="ct">{card.title}</span><span class="dot" aria-hidden="true"></span></span>
            <span class="lines">
              {#each card.lines as l (l)}
                <span class="line"><Icon name="check" size={16} stroke={2} style="color:var(--brand-ink)" />{l}</span>
              {/each}
            </span>
          </label>
        {/each}
      </div>
      <div class="foot">
        <button class="primary go" onclick={() => store.finishOnboarding()}>Continua</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dlg { width: min(840px, 100%); max-height: 100%; overflow: auto; box-sizing: border-box; padding: 32px 36px 28px; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); }
  h1 { margin: 24px 0 0; font-size: 28px; font-weight: 600; line-height: 1.2; }
  .intro { margin: 8px 0 0; max-width: 56ch; font-size: 15px; line-height: 1.5; color: var(--ink-2); }
  .cards { margin-top: 24px; gap: 16px; max-width: none; }
  .pic { display: block; padding: 10px; border-radius: 4px; background: var(--ground); }
  .pic svg { display: block; width: 100%; height: auto; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .ct { font-size: 17px; font-weight: 600; }
  .lines { display: flex; flex-direction: column; gap: 7px; }
  .line { display: flex; align-items: center; gap: 8px; font-size: 14px; color: var(--ink-2); }
  .foot { margin-top: 26px; display: flex; align-items: center; justify-content: flex-end; }
  .go { height: 36px; padding: 0 24px; border: 0; border-radius: 4px; background: var(--brand); color: var(--on-brand); font-size: 14.5px; font-weight: 600; }
</style>
