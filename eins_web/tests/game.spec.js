import { test, expect } from '@playwright/test';

test('home-screen installation exposes the card icons and standalone metadata', async ({ page, request }) => {
  await page.goto('/');
  await expect(page.locator('meta[name="apple-mobile-web-app-capable"]')).toHaveAttribute('content', 'yes');
  await expect(page.locator('meta[name="apple-mobile-web-app-title"]')).toHaveAttribute('content', 'Eins');
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute('crossorigin', 'use-credentials');
  const manifestURL = await page.locator('link[rel="manifest"]').getAttribute('href');
  const response = await request.get(manifestURL);
  expect(response.ok()).toBe(true);
  const manifest = await response.json();
  expect(manifest).toMatchObject({ name: 'Eins', start_url: '/', scope: '/', display: 'standalone' });
  expect(manifest.icons.map(icon => icon.sizes)).toEqual(expect.arrayContaining(['192x192', '512x512']));
  const appleIcon = await page.locator('link[rel="apple-touch-icon"]').getAttribute('href');
  await expect(page.locator('link[rel="apple-touch-icon"]')).toHaveAttribute('sizes', '1024x1024');
  const icons = [...manifest.icons, { src: appleIcon, sizes: '1024x1024' }];
  for (const icon of icons) {
    const image = await request.get(icon.src);
    expect(image.ok()).toBe(true);
    expect(image.headers()['content-type']).toContain('image/png');
    const size = await page.evaluate(async src => {
      const image = new Image();
      image.src = src;
      await image.decode();
      return `${image.naturalWidth}x${image.naturalHeight}`;
    }, icon.src);
    expect(size).toBe(icon.sizes);
  }
});

test('two browsers share a private table, moves, and the result', async ({ browser, baseURL }) => {
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  try {
    const host = await hostContext.newPage();
    const guest = await guestContext.newPage();
    const suffix = crypto.randomUUID().slice(0, 8);
    const names = [`Host-${suffix}`, `Guest-${suffix}`];
    const pages = [host, guest];
    const errors = [];
    for (const [i, page] of pages.entries()) {
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(baseURL);
      await page.getByLabel('What should we call you?').fill(names[i]);
      await page.getByRole('button', { name: 'Find a table' }).click();
      await expect(page.getByRole('heading', { name: 'The lobby' })).toBeVisible();
    }

    await host.getByRole('button', { name: 'Create private table' }).click();
    await host.getByLabel('Game code', { exact: true }).fill('friends');
    await host.getByRole('button', { name: 'Create table', exact: true }).click();
    await expect(host.getByRole('button', { name: 'Start game' })).toBeDisabled();
    await guest.locator('.table-row').filter({ hasText: names[0] }).getByRole('button', { name: 'Join table' }).click();
    await guest.getByLabel('Game code', { exact: true }).fill('wrong');
    await guest.getByRole('button', { name: 'Join with code' }).click();
    await expect(guest.getByRole('alert')).toContainText('Wrong game code');
    await guest.getByLabel('Game code', { exact: true }).fill('friends');
    await guest.getByRole('button', { name: 'Join with code' }).click();
    // These updates must arrive over SSE, without reloading either page.
    await expect(host.getByRole('button', { name: 'Start game' })).toBeEnabled();
    await expect(guest.getByRole('button', { name: 'Start game' })).toHaveCount(0);
    await host.getByRole('button', { name: 'Start game' }).click();
    for (const page of pages) await expect(page.getByRole('button', { name: /^Play / })).toHaveCount(7);
    await guest.reload();
    await expect(guest.getByRole('button', { name: /^Play / })).toHaveCount(7);

    // Inspect the public game API to check exact hand counts after a draw, not just a changed screen.
    const view = page => page.evaluate(async () => {
      const response = await fetch('/api/game/me', { headers: { code: sessionStorage.getItem('eins.player-code') } });
      return response.json();
    });
    const first = await view(host);
    const active = first.turn.current_player === first.players[0].id ? host : guest;
    const waiting = active === host ? guest : host;
    await expect(waiting.getByRole('button', { name: /^Draw / })).toBeDisabled();
    await expect(waiting.locator('.hand button:enabled')).toHaveCount(0);
    const beforeDraw = await view(active);
    const expectedCount = beforeDraw.hand.length + Math.max(1, beforeDraw.turn.pending_draw);
    await active.getByRole('button', { name: /^Draw / }).click();
    await expect(active.getByRole('button', { name: /^Play / })).toHaveCount(expectedCount);
    await expect(waiting.getByRole('heading', { name: 'Your move.', exact: true })).toBeVisible();

    // Find moves through the API: the UI must not identify legal cards for the player.
    let playedWild = false;
    let rejectedCard = false;
    for (let turn = 0; turn < 108 && (!playedWild || !rejectedCard); turn++) {
      const state = await view(host);
      const player = state.turn.current_player === state.players[0].id ? host : guest;
      const other = player === host ? guest : host;
      await expect(player.getByRole('button', { name: /^Draw / })).toBeEnabled();
      const hand = await view(player);
      const cards = player.getByRole('button', { name: /^Play / });
      await expect(player.locator('.hand button:enabled')).toHaveCount(hand.hand.length);
      // Draw animations temporarily hide their destination cards. Compare the
      // resting hand so animation progress is not mistaken for a legal-card hint.
      await expect(player.locator('.card-flight')).toHaveCount(0);
      const appearances = await cards.evaluateAll(cards => cards.map(card => {
        const style = getComputedStyle(card);
        return [style.opacity, style.boxShadow, style.borderColor];
      }));
      expect(new Set(appearances.map(style => JSON.stringify(style))).size).toBe(1);
      const invalid = hand.hand.findIndex(card => !hand.turn.playable.includes(card.id));
      if (!rejectedCard && invalid >= 0) {
        const card = cards.nth(invalid);
        const restingShadow = await card.evaluate(card => getComputedStyle(card).boxShadow);
        // Both pointer and repeated keyboard attempts give temporary feedback without a move.
        for (const reducedMotion of ['no-preference', 'reduce']) {
          await player.emulateMedia({ reducedMotion });
          if (reducedMotion === 'reduce') await card.press('Enter');
          else await card.click();
          await expect(card).toHaveCSS('box-shadow', /rgb\(201, 45, 38\)/);
          if (reducedMotion === 'reduce') await expect(card).toHaveCSS('translate', '0px');
          else await expect(card).not.toHaveCSS('translate', /^(none|0px)$/);
          await expect(card).toHaveCSS('box-shadow', restingShadow);
          expect(await view(player)).toEqual(hand);
          await expect(player.getByRole('group', { name: 'Choose the next color' })).toHaveCount(0);
        }
        await player.emulateMedia({ reducedMotion: 'no-preference' });
        rejectedCard = true;
      }
      const wildIndex = hand.hand.findIndex(card => card.card.Wild && hand.turn.playable.includes(card.id));
      if (!playedWild && wildIndex >= 0) {
        const wild = cards.nth(wildIndex);
        const handCount = await player.getByRole('button', { name: /^Play / }).count();
        await player.getByRole('button', { name: 'Call Eins!', exact: true }).click();
        await wild.click();
        await expect(player.getByRole('group', { name: 'Choose the next color' })).toBeVisible();
        await player.keyboard.press('Escape');
        await expect(player.getByRole('group', { name: 'Choose the next color' })).toHaveCount(0);
        await wild.click();
        await player.getByRole('button', { name: 'Blue', exact: true }).click();
        await expect(player.getByRole('button', { name: /^Play / })).toHaveCount(handCount - 1);
        await expect(player.getByRole('button', { name: 'Call Eins!', exact: true })).toHaveAttribute('aria-pressed', 'false');
        await expect(other.locator('.active-color')).toHaveText('Blue');
        expect((await view(other)).turn.color).toBe('Blue');
        playedWild = true;
      } else {
        await player.getByRole('button', { name: /^Draw / }).click();
        await expect(other.getByRole('heading', { name: 'Your move.', exact: true })).toBeVisible();
      }
    }
    expect(playedWild).toBe(true);
    expect(rejectedCard).toBe(true);

    let playedNormal = false;
    for (let turn = 0; turn < 108 && !playedNormal; turn++) {
      const state = await view(host);
      const player = state.turn.current_player === state.players[0].id ? host : guest;
      const other = player === host ? guest : host;
      await expect(player.getByRole('button', { name: /^Draw / })).toBeEnabled();
      const hand = await view(player);
      const cardIndex = hand.hand.findIndex(card => card.card.Normal && hand.turn.playable.includes(card.id));
      if (cardIndex >= 0) {
        const card = player.getByRole('button', { name: /^Play / }).nth(cardIndex);
        const label = (await card.getAttribute('aria-label')).replace('Play ', 'Top card: ');
        const count = await player.getByRole('button', { name: /^Play / }).count();
        await card.click();
        await expect(player.getByRole('button', { name: /^Play / })).toHaveCount(count - 1);
        await expect(other.locator('.discard')).toHaveAttribute('aria-label', label);
        playedNormal = true;
      } else {
        await player.getByRole('button', { name: /^Draw / }).click();
        await expect(other.getByRole('heading', { name: 'Your move.', exact: true })).toBeVisible();
      }
    }
    expect(playedNormal).toBe(true);

    // A peer leaving ends the two-player game, then both can return to the lobby.
    await guest.getByRole('button', { name: 'Leave game', exact: true }).click();
    await expect(host.getByRole('heading', { name: 'You win!', exact: true })).toBeVisible();
    await host.getByRole('button', { name: 'Back to lobby' }).click();
    for (const page of pages) await expect(page.getByRole('heading', { name: 'The lobby' })).toBeVisible();
    expect(errors).toEqual([]);
  } finally {
    await hostContext.close();
    await guestContext.close();
  }
});

test('a session no longer known by the server can register again', async ({ page }) => {
  await page.addInitScript(() => {
    sessionStorage.setItem('eins.player-code', '00000000-0000-4000-8000-000000000000');
  });
  await page.goto('/');
  await expect(page.getByRole('alert')).toContainText('Your session has ended');
  await expect(page.getByLabel('What should we call you?')).toBeVisible();
  expect(await page.evaluate(() => sessionStorage.getItem('eins.player-code'))).toBeNull();
  await page.getByLabel('What should we call you?').fill(`Returning-${crypto.randomUUID().slice(0, 8)}`);
  await page.getByRole('button', { name: 'Find a table' }).click();
  await expect(page.getByRole('heading', { name: 'The lobby' })).toBeVisible();
});

test('an empty proxy failure is visible and registration can be retried', async ({ page }) => {
  await page.route('**/api/player/register', route => route.fulfill({ status: 503, body: '' }));
  await page.goto('/');
  await page.getByLabel('What should we call you?').fill(`Retry-${crypto.randomUUID().slice(0, 8)}`);
  await page.getByRole('button', { name: 'Find a table' }).click();
  await expect(page.getByRole('alert')).toContainText('503');
  await page.unroute('**/api/player/register');
  await page.getByRole('button', { name: 'Find a table' }).click();
  await expect(page.getByRole('heading', { name: 'The lobby' })).toBeVisible();
});

for (const width of [1280, 390]) {
  test(`confirmed cards travel between their actual positions at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const me = '00000000-0000-4000-8000-000000000001';
    const game = '00000000-0000-4000-8000-000000000002';
    const normal = (id, color, symbol) => ({ id, card: { Normal: { color, symbol } } });
    const state = {
      game_id: game, state: 'Running', direction: 'Clockwise', winner: null,
      players: [{ id: me, nick: 'Motion test', card_count: 3 }],
      hand: [normal(18, 'Red', 'DrawTwo'), normal(35, 'Blue', 'Two'), { id: 0, card: { Wild: { symbol: 'ChooseColor' } } }],
      top_card: { Normal: { color: 'Red', symbol: 'DrawTwo' } },
      turn: { current_player: me, color: 'Red', pending_draw: 2, playable: [18, 0] },
    };
    let reject = false;
    await page.addInitScript(() => sessionStorage.setItem('eins.player-code', 'motion-test'));
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname;
      if (path === '/api/events') return route.abort();
      if (path === '/api/player/me') return route.fulfill({ json: { player: {
        id: me, nick: 'Motion test', code: 'motion-test', last_activity: '2026-01-01T00:00:00Z', state: { GameJoined: game },
      } } });
      if (path === '/api/game/me') return route.fulfill({ json: state });
      if (reject) return route.fulfill({ status: 409, body: 'Move rejected' });
      if (path === '/api/game/play') {
        const move = route.request().postDataJSON();
        state.top_card = state.hand.find(card => card.id === move.card).card;
        state.hand = state.hand.filter(card => card.id !== move.card);
        if (move.color) state.turn.color = move.color;
        if (move.card === 18) state.turn.pending_draw = 4;
      } else if (path === '/api/game/draw') {
        state.hand.push(...[58, 59, 60, 61].map((id, i) => normal(id, 'Orange', ['Zero', 'One', 'Two', 'Three'][i])));
        state.turn.pending_draw = 0;
      }
      state.players[0].card_count = state.hand.length;
      return route.fulfill({ json: null });
    });
    await page.goto('/');
    await expect(page.locator('.hand .card')).toHaveCount(3);
    await expect(page.locator('.card-flight')).toHaveCount(0);

    // Freeze only the visual timeline, not requests or Yew, so endpoint geometry
    // can be checked independently of machine speed and the easing curve.
    const arm = () => page.evaluate(() => {
      window.flightsSeen = 0;
      window.motionObserver = new MutationObserver(() => {
        for (const card of document.querySelectorAll('.card-flight')) {
          for (const animation of card.getAnimations()) {
            if (animation.playState === 'running') {
              animation.pause();
              animation.currentTime = 0;
              window.flightsSeen++;
            }
          }
        }
      });
      window.motionObserver.observe(document.body, { childList: true });
    });
    const center = locator => locator.evaluate(element => {
      const r = element.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    });
    const checkTravel = async (from, target, count) => {
      await expect.poll(() => page.evaluate(() => window.flightsSeen)).toBe(count);
      const flight = page.locator('.card-flight');
      const positions = await flight.evaluateAll(cards => cards.filter(card => card.getAnimations().length).map(card => {
        const a = card.getAnimations()[0];
        const position = () => { const r = card.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; };
        const start = position();
        const { delay, duration } = a.effect.getTiming();
        a.currentTime = delay + duration * 0.35;
        const middle = position();
        a.currentTime = delay + duration - 0.01;
        return { start, middle, end: position(), hidden: card.getAttribute('aria-hidden') };
      }));
      const to = await center(target);
      for (const position of positions) {
        expect(position.hidden).toBe('true');
        expect(Math.hypot(position.start.x - from.x, position.start.y - from.y)).toBeLessThan(2);
        expect(Math.hypot(position.middle.x - from.x, position.middle.y - from.y)).toBeGreaterThan(20);
      }
      const last = positions.at(-1).end;
      expect(Math.hypot(last.x - to.x, last.y - to.y)).toBeLessThan(2);
      await page.evaluate(() => {
        window.motionObserver.disconnect();
        for (const card of document.querySelectorAll('.card-flight')) for (const animation of card.getAnimations()) animation.finish();
      });
      await expect(page.locator('.card-flight')).toHaveCount(0);
      await expect(target).not.toHaveCSS('opacity', '0');
    };

    const red = page.getByRole('button', { name: 'Play Red +2', exact: true });
    // A failed request must not invent a successful play or leave a hidden card.
    reject = true;
    await red.click();
    await expect(page.getByRole('alert')).toContainText('Move rejected');
    await expect(page.locator('.card-flight')).toHaveCount(0);
    await expect(red).toBeVisible();
    reject = false;
    await page.getByRole('button', { name: 'Retry refresh' }).click();
    await expect(page.getByRole('alert')).toHaveCount(0);
    await page.mouse.move(0, 0);
    await expect(red).toHaveCSS('transform', 'none');
    await expect(red).toBeEnabled();
    await arm();
    const origin = await center(red);
    await red.press('Enter');
    await checkTravel(origin, page.locator('.discard'), 1);
    await expect(page.locator('.discard')).toHaveAttribute('aria-label', 'Top card: Red +2');

    await arm();
    const deck = await center(page.locator('.deck'));
    await page.getByRole('button', { name: 'Draw 4 cards', exact: true }).click();
    await checkTravel(deck, page.getByRole('button', { name: 'Play Orange 3', exact: true }), 4);
    await expect(page.locator('.hand .card')).toHaveCount(6);

    // Choosing/cancelling a wild is not a play; reduced motion skips travel entirely.
    await page.getByRole('button', { name: 'Play Wild ✦', exact: true }).click();
    await expect(page.locator('.card-flight')).toHaveCount(0);
    await page.keyboard.press('Escape');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await arm();
    await page.getByRole('button', { name: 'Play Wild ✦', exact: true }).click();
    await page.getByRole('button', { name: 'Green', exact: true }).click();
    await expect(page.locator('.hand .card')).toHaveCount(5);
    await expect(page.locator('.active-color')).toHaveText('Green');
    expect(await page.evaluate(() => window.flightsSeen)).toBe(0);
    await expect(page.locator('.card-flight')).toHaveCount(0);
  });
}
