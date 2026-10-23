import { describe, it, expect } from 'vitest';
import { ACCENT_SWATCHES, accentPalette, accentShift, contrast, normalizeHex } from '../src/lib/accent';

// Colori scelti apposta per mettere alla prova i casi difficili: chiarissimi,
// scurissimi, grigi, saturi fuori dai colori dello schermo.
const HARD = ['#FFFFFF', '#000000', '#FFFF00', '#FFD700', '#00FF00', '#00FFFF', '#FF00FF', '#FF0000', '#808080', '#1A1A1A', '#F7630C', '#7FBA00', '#FFB900', '#E1DFDD'];
const ALL = [...ACCENT_SWATCHES.map((s) => s.hex), ...HARD];

describe('colore di accento', () => {
  it('riconosce i colori scritti a mano', () => {
    expect(normalizeHex('#0f6cbd')).toBe('#0F6CBD');
    expect(normalizeHex('abc')).toBe('#AABBCC');
    expect(normalizeHex(' #12AB9f ')).toBe('#12AB9F');
    expect(normalizeHex('blu')).toBeNull();
    expect(normalizeHex('#12345')).toBeNull();
  });

  it.each(ALL)('%s resta leggibile nel tema chiaro', (hex) => {
    const p = accentPalette(hex, 'chiaro');
    expect(contrast(p['--brand']!, p['--on-brand']!)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--head']!, p['--on-head']!)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--brand-ink']!, '#F0F0F0')).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--brand-ink']!, p['--brand-soft']!)).toBeGreaterThanOrEqual(4.5);
    // Testo normale sulle righe selezionate.
    expect(contrast('#242424', p['--brand-sel']!)).toBeGreaterThanOrEqual(4.5);
    // I pulsanti si distinguono dal foglio bianco.
    expect(contrast(p['--brand']!, '#FFFFFF')).toBeGreaterThanOrEqual(1.4);
  });

  it.each(ALL)('%s resta leggibile nel tema scuro', (hex) => {
    const p = accentPalette(hex, 'scuro');
    expect(contrast(p['--brand']!, p['--on-brand']!)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--brand-ink']!, '#292929')).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--brand-ink']!, p['--brand-soft']!)).toBeGreaterThanOrEqual(4.5);
    expect(contrast('#FFFFFF', p['--brand-sel']!)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(p['--brand']!, '#292929')).toBeGreaterThanOrEqual(1.4);
    // In scuro la barra in alto resta grigia: non si tocca.
    expect(p['--head']).toBeUndefined();
  });

  it('i colori proposti restano quasi identici a quelli scelti', () => {
    for (const { hex } of ACCENT_SWATCHES) {
      const p = accentPalette(hex, 'chiaro');
      expect(p['--brand']).toBe(hex);
      expect(accentShift(hex, p['--brand']!)).toBeLessThan(0.01);
    }
  });

  it('con il blu predefinito i colori sono quelli di Fluent 2', () => {
    const light = accentPalette('#0F6CBD', 'chiaro');
    expect(light['--brand']).toBe('#0F6CBD');
    expect(light['--brand-ink']).toBe('#0F6CBD');
    expect(light['--on-brand']).toBe('#FFFFFF');
    expect(accentShift('#EBF3FC', light['--brand-soft']!)).toBeLessThan(0.02);
    expect(accentShift('#CFE4FA', light['--brand-sel']!)).toBeLessThan(0.03);
    const dark = accentPalette('#0F6CBD', 'scuro');
    expect(accentShift('#115EA3', dark['--brand']!)).toBeLessThan(0.03);
    expect(accentShift('#479EF5', dark['--brand-ink']!)).toBeLessThan(0.08);
  });

  it('un giallo acceso diventa un fondo con testo scuro', () => {
    const p = accentPalette('#FFD700', 'chiaro');
    expect(p['--on-brand']).toBe('#1A1A1A');
    expect(p['--head-hover']).toContain('0, 0, 0');
  });
});

describe('scelta del colore nelle impostazioni', () => {
  it('salva il colore scelto e torna al blu', async () => {
    const { MailStore } = await import('../src/lib/store.svelte');
    const { MockBackend } = await import('../src/lib/mockBackend');
    const s = new MailStore({ persist: false, backend: new MockBackend() });
    expect(s.accent).toBeNull();
    s.setAccent('#038387');
    expect(s.accent).toBe('#038387');
    s.setAccent('c4314b');
    expect(s.accent).toBe('#C4314B');
    // Un codice sbagliato non cambia nulla.
    s.setAccent('#12');
    expect(s.accent).toBe('#C4314B');
    // Il blu predefinito equivale a nessuna scelta.
    s.setAccent('#0f6cbd');
    expect(s.accent).toBeNull();
    s.setAccent('#038387');
    s.setAccent(null);
    expect(s.accent).toBeNull();
  });
});
