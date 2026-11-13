// Layout-only effects: Rust and the server remain responsible for game state.
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const running = new Set();
// The hand keeps its earlier height while cards fly. A shorter page would scroll
// on its own, and that scroll would cancel the flights it moved.
let heldHand = null;

function releaseHand() {
  if (heldHand) heldHand.style.minHeight = '';
  heldHand = null;
}
// Opponents' cards travel to and from their seat at this fraction of full size.
const SEAT_SCALE = 0.4;

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

// A card-sized point at the centre of an opponent's seat.
function seat(element, card) {
  const { x, y } = snapshot(element);
  return { x, y, width: card.width * SEAT_SCALE, height: card.height * SEAT_SCALE };
}

function opponents() {
  return new Map([...document.querySelectorAll('.players [data-player]')]
    .map(player => [player.dataset.player, player]));
}

export function capture_cards() {
  if (reducedMotion.matches) return null;
  return {
    hand: new Map([...document.querySelectorAll('.hand [data-card]')]
      .map(card => [card.dataset.card, snapshot(card)])),
    discard: snapshot(document.querySelector('.discard')),
    handHeight: document.querySelector('.hand').offsetHeight,
    counts: new Map([...opponents()].map(([id, player]) => [id, Number(player.dataset.count)])),
  };
}

function overlay(face, at, size = at) {
  // A decorative div, never a second playable button or screen-reader entry.
  const card = document.createElement('div');
  card.className = face.className;
  card.classList.remove('rejected', 'discard');
  card.classList.add('card-flight');
  card.setAttribute('aria-hidden', 'true');
  card.innerHTML = face.innerHTML;
  Object.assign(card.style, {
    left: `${at.x - size.width / 2}px`, top: `${at.y - size.height / 2}px`,
    width: `${size.width}px`, height: `${size.height}px`,
    transform: face.classList.contains('discard') ? 'rotate(5deg)' : 'none',
  });
  document.body.append(card);
  return card;
}

// Moves a copy of `face` from `from` to `to`, both card-sized points with an
// optional rotation. `size` is the copy's layout size, so scaled-down ends keep
// borders and text in proportion. `target` stays hidden until the copy lands.
// `reveal` starts the copy face down and turns it over early in flight.
function fly(face, from, to, { size = to, target, reveal, fade, delay = 0, under, hand = false }) {
  const card = overlay(face, to, size);
  if (!hand) card.classList.remove('hand-card');
  const below = under ? overlay(under, to) : null;
  if (below) below.style.zIndex = '99';
  const duration = 460;
  const timing = { duration, delay, fill: 'both', easing: 'cubic-bezier(.22,.7,.24,1)' };
  const at = point => `rotate(${point.rotate ?? 0}deg) scale(${point.width / size.width}, ${point.height / size.height})`;
  const start = `translate(${from.x - to.x}px, ${from.y - to.y}px) ${at(from)}`;
  const end = `translate(0, 0) ${at(to)}`;
  // Cards joining an opponent's hand disappear into their seat.
  const keyframes = fade
    ? [{ transform: start }, { opacity: 1, offset: 0.75 }, { transform: end, opacity: 0 }]
    : [{ transform: start }, { transform: end }];
  const flight = card.animate(keyframes, timing);
  const effects = [];
  if (target) {
    effects.push(target.animate([{ opacity: 0 }, { opacity: 0 }], {
      duration: duration + delay, fill: 'both',
    }));
  }
  if (reveal) {
    const back = document.createElement('div');
    back.className = 'flight-back';
    back.textContent = 'eins.';
    card.append(back);
    effects.push(back.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 150, delay: delay + 100, fill: 'both',
    }));
  }
  const finish = () => {
    running.delete(finish);
    if (!running.size) releaseHand();
    card.remove();
    below?.remove();
    flight.cancel();
    for (const effect of effects) effect.cancel();
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
  // Players who left are no longer shown, so their cards go nowhere.
  const seats = [...opponents()]
    .filter(([id]) => before.counts.has(id))
    .map(([id, player]) => ({ player, change: Number(player.dataset.count) - before.counts.get(id) }));
  const played = seats.filter(seat => seat.change < 0);
  const drew = seats.filter(seat => seat.change > 0);
  // Repeated SSE refreshes must not restart or cancel an in-progress flight.
  if (!added.length && !removed.length && !played.length && !drew.length) return;
  cancel_card_motion();
  // Hold before anything reads layout: the first layout would already clamp the scroll.
  heldHand = document.querySelector('.hand');
  heldHand.style.minHeight = `${before.handHeight}px`;
  const discard = document.querySelector('.discard');
  const deck = snapshot(document.querySelector('.deck'));
  const pile = { ...snapshot(discard), rotate: 5 };
  // Only the newest top card is known, so show a play only when it is the one
  // card that reached the pile since the last update.
  const plays = removed.length - played.reduce((sum, seat) => sum + seat.change, 0);
  if (plays === 1 && removed.length) {
    const [, card] = removed[0];
    fly(card.face, card, pile, { target: discard, under: before.discard.face });
  } else if (plays === 1) {
    fly(pile.face, seat(played[0].player, pile), pile, {
      target: discard, reveal: true, under: before.discard.face,
    });
  }
  // Keep even large penalties brisk, while dealing each card separately.
  const deals = added.length + drew.reduce((sum, seat) => sum + seat.change, 0);
  const delay = index => index * Math.min(85, 600 / deals);
  const from = { ...deck, rotate: -7 };
  added.forEach(([, card], index) => {
    fly(card, from, snapshot(card), { target: card, reveal: true, delay: delay(index), hand: true });
  });
  let index = added.length;
  for (const { player, change } of drew) {
    const to = seat(player, deck);
    for (let i = 0; i < change; i++) {
      // The deck's own face is a card back, so these stay face down.
      fly(deck.face, from, to, { size: deck, fade: true, delay: delay(index++) });
    }
  }
  if (!running.size) releaseHand();
}

// Aims each Eins speech bubble's tail at the seat of the player who called it:
// it leaves the edge facing the seat, slides toward it, and tilts its tip at it.
export function point_eins_bubbles() {
  const seats = document.querySelectorAll('.players .player');
  const clamp = (value, low, high) => (low > high ? (low + high) / 2 : Math.min(high, Math.max(low, value)));
  for (const bubble of document.querySelectorAll('.eins-bubble')) {
    const seat = seats[bubble.dataset.seat];
    const path = bubble.querySelector('.eins-tail path');
    if (!seat) {
      path.removeAttribute('d');
      continue;
    }
    // Layout geometry, so the pop-in scale does not skew the measurement.
    const width = bubble.offsetWidth;
    const height = bubble.offsetHeight;
    const parent = bubble.offsetParent.getBoundingClientRect();
    const left = parent.x + bubble.offsetLeft;
    const top = parent.y + bubble.offsetTop;
    const target = seat.getBoundingClientRect();
    const seatX = target.x + target.width / 2 - left;
    const seatY = target.y + target.height / 2 - top;
    const dx = seatX - width / 2;
    const dy = seatY - height / 2;
    // The tail's base sits on the inside of the border so its fill hides it.
    const radius = parseFloat(getComputedStyle(bubble).borderTopLeftRadius);
    const border = 3;
    const half = 12;
    const sideways = Math.abs(dx) * height > Math.abs(dy) * width;
    const [normal, tangent] = sideways ? [[Math.sign(dx), 0], [0, 1]] : [[0, Math.sign(dy) || -1], [1, 0]];
    const base = sideways
      ? [dx > 0 ? width - border : border, clamp(height / 2 + dy * (width / 2) / Math.abs(dx), radius + half, height - radius - half)]
      : [clamp(width / 2 + dx * (height / 2) / Math.abs(dy || 1), radius + half, width - radius - half), dy > 0 ? height - border : border];
    const toSeat = [seatX - base[0], seatY - base[1]];
    // Lean toward the seat, but never so far that the tail lies along the edge.
    const lean = clamp(Math.atan2(
      toSeat[0] * tangent[0] + toSeat[1] * tangent[1],
      toSeat[0] * normal[0] + toSeat[1] * normal[1],
    ), -0.9, 0.9);
    const reach = 20 + border;
    const tip = [0, 1].map(i => base[i] + (normal[i] * Math.cos(lean) + tangent[i] * Math.sin(lean)) * reach);
    const corner = sign => [0, 1].map(i => base[i] + tangent[i] * half * sign);
    // The SVG covers the bubble's border box, so these are its coordinates too.
    const point = ([x, y]) => `${x} ${y}`;
    path.setAttribute('d', `M ${point(corner(-1))} L ${point(tip)} L ${point(corner(1))}`);
    bubble.style.transformOrigin = `${tip[0]}px ${tip[1]}px`;
  }
}
