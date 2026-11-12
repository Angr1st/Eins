# Eins in the browser

A Rust/WebAssembly client built with Yew. It depends directly on `eins_lib` for
API messages, player state, card definitions, and game views. The server runs the
game rules and sends each player their own hand and legal moves; the browser
does not maintain a second implementation of those rules. The terminal client
can join the same games without changes.

## Run locally

Use the Rust version in `../mise.toml`. Install the browser build tools once:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
```

From the repository root, start the existing server:

```sh
cargo run -p eins_server --locked
```

In a second terminal, from `eins_web`, start the web client:

```sh
trunk serve --locked
```

Open `http://127.0.0.1:8082` on your local machine. Trunk proxies `/api/` to the
server on port 8080, stripping that prefix. There is no CORS configuration and
the existing server routes stay unchanged. Use separate tabs or browsers for
multiple players. Duplicating an existing tab may copy its session; open a fresh
tab instead.

Register a nickname, create or join an open/private table, then let the host
start. Cards do not reveal which moves are legal. Trying an illegal card briefly
shakes it and outlines it in red (outline only with reduced motion enabled).
Wild cards open a color chooser; Escape
cancels. Call Eins before playing down to one card. Draw and leave use the same
server rules as the terminal client.

Confirmed plays slide from your hand onto the discard pile. Drawn cards travel
from the deck into your hand, with staggered arrivals for penalties. These
effects respect reduced motion and never delay game-state updates.

## Preview in an Amp orb

`.agents/setup` installs the toolchain and dependencies. `.amp/services.yaml`
declares the API and release-mode web preview as supervised services. From the
repository root, start or repair both with:

```sh
amp orb services ensure
```

Open the printed **Eins web client** portal URL, or use the Portal tab. Only the
web client has a portal; its `/api/` proxy reaches the local API. Readiness checks
verify the API directly and through that proxy. No resume script is needed.
Generated portal URLs remain local to each orb and are ignored by Git.

## Sessions and live updates

The player credential is stored in **sessionStorage**, scoped to the current
origin and tab, so a reload restores the seat. It is sent only in the existing
`code` header, including for the fetch-based server-sent event stream. It is
never put in URLs. Disconnects reconnect automatically and refresh the view;
expired sessions return to registration. Requests are serialized and incoming
refresh notifications are coalesced so an older response cannot overwrite a
newer move. Failed mutations are not automatically retried.

Closing a tab does **not immediately** leave the table. Leave explicitly first;
the server removes disconnected idle players after about ten minutes. An open
event stream keeps a player active. Server state is in memory, so a server
restart ends all sessions and games.

## Build and host

```sh
trunk build --release --locked
```

Serve the generated `dist/` at the root of an HTTPS origin. Configure that same
origin to proxy `/api/*` to `eins_server`, removing `/api`. Forward the `code`
header; disable response buffering for `/api/events` and allow long-lived SSE
responses. Do not cache API responses. Serve `.wasm` as `application/wasm`.
Trunk's proxy is for development, not a production deployment.

Serve `.webmanifest` as `application/manifest+json`. Keep the manifest link's
`crossorigin="use-credentials"` so it can load behind authentication such as
Cloudflare Access, even when hosted on the same origin.

The UI uses Google Fonts with local sans-serif fallbacks. Node is only needed
for browser tests, not to build or run the client.

## Verify

From the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo clippy -p eins_web --target wasm32-unknown-unknown --locked -- -D warnings
```

From `eins_web`:

```sh
npm ci
npx playwright install chromium
npm test
```

Browser tests start the real Rust server and Trunk, or reuse local servers
already running on the configured ports. Set `CHROME_PATH` to use an installed
Chromium executable instead of downloading one. Tests cover two independent
players, private-code rejection, pushed updates, drawing, wild color selection,
Eins, reload recovery, leaving, and expired sessions.

## Add to the iPhone home screen

Open the HTTPS site in Safari, tap **Share → Add to Home Screen**, and keep
**Open as Web App** enabled if offered. Eins uses a three-card icon and opens
in standalone mode. If an existing shortcut still shows the old icon, remove
that shortcut and add it again after the updated site is deployed.
The game still requires a network connection; there is no offline game cache.

The icon source is `icons/icon.svg`. The opaque PNG exports let iOS apply its
own rounded corners and support iOS versions before SVG icon support in iOS 26.
The Apple icon is rendered at 1024×1024 so larger previews do not upscale a
small bitmap. After editing the SVG, regenerate them with uv, CairoSVG,
and ImageMagick (CairoSVG preserves the SVG transforms):

```sh
uvx --from cairosvg cairosvg icons/icon.svg -f png -s 4 | magick - -resize 512x512 -alpha off PNG24:icons/icon-512.png
magick icons/icon-512.png -resize 192x192 PNG24:icons/icon-192.png
uvx --from cairosvg cairosvg icons/icon.svg -f png -s 4 | magick - -resize 1024x1024 -alpha off PNG24:icons/apple-touch-icon.png
```
