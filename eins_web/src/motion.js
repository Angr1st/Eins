// Layout-only effects: Rust and the server remain responsible for game state.
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const running = new Set();

export function cancel_card_motion() {
  for (const finish of [...running]) finish();
}

// Fixed-position flights must not outlive the layout they were measured against.
addEventListener('resize', cancel_card_motion);
addEventListener('scroll', cancel_card_motion, { passive: true, capture: true });
reducedMotion.addEventListener('change', cancel_card_motion);

function snapshot(element) {
  const rect = element.getBoundingClientRect();
  return {
    x: rect.x + rect.width / 2,
    y: rect.y + rect.height / 2,
    width: element.offsetWidth,
    height: element.offsetHeight,
    face: element.cloneNode(true),
  };
}

export function capture_cards() {
  if (reducedMotion.matches) return null;
  return {
    hand: new Map([...document.querySelectorAll('.hand [data-card]')]
      .map(card => [card.dataset.card, snapshot(card)])),
    discard: snapshot(document.querySelector('.discard')),
  };
}

function overlay(source, at) {
  // A decorative div, never a second playable button or screen-reader entry.
  const card = document.createElement('div');
  card.className = source.face.className;
  card.classList.remove('rejected', 'discard');
  card.classList.add('card-flight');
  card.setAttribute('aria-hidden', 'true');
  card.innerHTML = source.face.innerHTML;
  Object.assign(card.style, {
    left: `${at.x - at.width / 2}px`, top: `${at.y - at.height / 2}px`,
    width: `${at.width}px`, height: `${at.height}px`,
    transform: source.face.classList.contains('discard') ? 'rotate(5deg)' : 'none',
  });
  document.body.append(card);
  return card;
}

function fly(source, from, target, drawing, delay, previousDiscard) {
  const to = snapshot(target);
  const card = overlay(source, to);
  if (!drawing) card.classList.remove('hand-card');
  const under = previousDiscard ? overlay(previousDiscard, to) : null;
  if (under) under.style.zIndex = '99';
  const duration = 460;
  const timing = { duration, delay, fill: 'both', easing: 'cubic-bezier(.22,.7,.24,1)' };
  const start = `translate(${from.x - to.x}px, ${from.y - to.y}px) rotate(${drawing ? -7 : 0}deg) scale(${from.width / to.width}, ${from.height / to.height})`;
  const end = `translate(0, 0) rotate(${drawing ? 0 : 5}deg) scale(1)`;
  const flight = card.animate([{ transform: start }, { transform: end }], timing);
  const hidden = target.animate([{ opacity: 0 }, { opacity: 0 }], {
    duration: duration + delay, fill: 'both',
  });
  let reveal;
  if (drawing) {
    const back = document.createElement('div');
    back.className = 'flight-back';
    back.textContent = 'eins.';
    card.append(back);
    reveal = back.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 150, delay: delay + 100, fill: 'both',
    });
  }
  const finish = () => {
    running.delete(finish);
    card.remove();
    under?.remove();
    flight.cancel();
    hidden.cancel();
    reveal?.cancel();
  };
  running.add(finish);
  flight.finished.then(finish, () => {});
}

export function animate_cards(before) {
  if (!before || reducedMotion.matches) return;
  const hand = new Map([...document.querySelectorAll('.hand [data-card]')]
    .map(card => [card.dataset.card, card]));
  const added = [...hand].filter(([id]) => !before.hand.has(id));
  const removed = [...before.hand].filter(([id]) => !hand.has(id));
  // Repeated SSE refreshes must not restart or cancel an in-progress flight.
  if (!added.length && !removed.length) return;
  cancel_card_motion();
  const discard = document.querySelector('.discard');
  if (removed.length === 1) {
    fly(removed[0][1], removed[0][1], discard, false, 0, before.discard);
  }
  const deck = snapshot(document.querySelector('.deck'));
  added.forEach(([, card], index) => {
    // Keep even large penalties brisk, while dealing each card separately.
    const delay = index * Math.min(85, 600 / added.length);
    fly(snapshot(card), deck, card, true, delay);
  });
}
