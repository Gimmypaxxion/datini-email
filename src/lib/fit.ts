// Barra dei comandi che si stringe bene: quando i gruppi non ci stanno, si
// nascondono a partire da quelli meno usati (attributo data-drop: 1 è il
// primo a sparire), come fa Outlook quando la finestra è stretta. I gruppi
// senza data-drop restano sempre.

export function fitGroups(el: HTMLElement) {
  let frame = 0;
  const fit = () => {
    frame = 0;
    const groups = [...el.querySelectorAll<HTMLElement>('[data-drop]')].sort((a, b) => Number(a.dataset.drop) - Number(b.dataset.drop));
    for (const g of groups) g.classList.remove('dropped');
    for (const g of groups) {
      if (el.scrollWidth <= el.clientWidth + 1) break;
      g.classList.add('dropped');
    }
  };
  const later = () => {
    if (!frame) frame = requestAnimationFrame(fit);
  };
  const ro = new ResizeObserver(later);
  ro.observe(el);
  // Cambiano scheda o modulo: cambiano i gruppi.
  const mo = new MutationObserver(later);
  mo.observe(el, { childList: true, subtree: true });
  fit();
  return {
    destroy() {
      cancelAnimationFrame(frame);
      ro.disconnect();
      mo.disconnect();
    },
  };
}
