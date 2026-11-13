# Native NixOS deployment on Hostinger

Eins uses native systemd services, Nginx, and Cloudflare Tunnel. No containers
or public HTTP listener are needed. This follows the existing Square deployment:
build a Linux release outside the Nix sandbox, then package it with
`autoPatchelfHook` so the server uses NixOS's loader and libraries. The host's
NixOS channel supplies Nginx and cloudflared.

| Unit | Listener | Purpose |
| --- | --- | --- |
| `eins` | `127.0.0.1:8080` | Rust game API and SSE |
| `eins-web` | `127.0.0.1:8082` | Static website and `/api/*` proxy |
| `eins-tunnel` | `127.0.0.1:20243` | Tunnel metrics, not the website |

These avoid the existing Grocer ports 4000/20241 and Square ports 8090/20242.
Confirm availability before activation. The module does not modify their
services, directories, credentials, routes, or firewall settings. All three
Eins services run as non-root dynamic users.

`package.nix` packages only the server binary and `web.nix` packages only the
compiled website, so a website-only update changes just the `eins-web` Nginx
root and leaves the game server running. Nginx serves `.webmanifest` as
`application/manifest+json`; `index.html` links it with
`crossorigin="use-credentials"` so browsers send the Access cookie when fetching it.

## Current deployment

First deployed on 9 November 2026 at **https://eins.angr1st.com**, restricted through
Cloudflare Access to **Torben and Alina**. Email OTP is the only login method,
sessions last 24 hours, and there is no bypass policy. The tunnel validates the
Eins Access application's JWT before forwarding requests to the origin.

- Release: `93f55e9ea5b4-archive-169789ea0336` (website from master
  `93f55e9ea5b417b16bf419d3aacf4dfd1637debb`, with the running backend retained).
- Archive SHA256: `169789ea03360f5bd84cf8445961d6f9423223105ae52a0d22a51c1b5ac581b5`.
- Active release: `/opt/eins/current`, a real directory holding the live
  `deployment/` and the verbatim archive in `archive/`. Release record:
  `/opt/eins/deployed-release`; staged copy under `/opt/eins/releases/`.
- Tunnel: `eins-hostinger`, ID `fa206955-46a7-4ac1-a2af-0bffbef432bc`.
- Access application: `Eins`, ID `78fb8f7b-06ef-42e0-b58c-7ed882c8ea51`.
- Host configuration backup: `/etc/nixos/configuration-before-eins.nix`.

Production uses this repository's packaging: the live `nixos.nix`, `package.nix`
and `web.nix` are byte-identical to the ones in `deployment/`. The migration to
the server-only `package.nix` happened on 13 November 2026 with release
`9cb80cf1d329-bundle-909a6b5e9957` (NixOS generation 14), which restarted `eins`
once at 07:28 UTC. The backend package is
`/nix/store/6mr06x83ck9541kgmzhynhfcj56sqhvq-eins-0.1.0`; its binary
(SHA256 `ce1d0742…`) is the one built for commit
`97173295ec121e1fd97b5a82304351cc25e1e57d`. Server and library source are
unchanged since then.

Later on 13 November 2026 (generation 15) the website was updated to release
`93f55e9ea5b4-archive-169789ea0336` by replacing only `bundle/public/` and
bumping `web.nix`'s version; the archive's own `eins_server` was not used. The
web package is `/nix/store/9nhd69jg2wbvjpy0ga94q6b9is97hm1r-eins-web-93f55e9ea5b4`.
A build comparison against `/run/current-system` showed only the `eins-web`
Nginx root changing, and the switch restarted only `eins-web`. The `eins` PID,
Grocer, Square, Mellow, tunnels, SSH, and firewall were unchanged; all ten
checked units were active with no failed units. All served website files
matched the release; origin health, MIME types, authentication headers, and
unbuffered SSE notifications passed. Anonymous public website and API requests
redirected to Eins's Access login; HTTP redirected to HTTPS.

Rollback: restore `/opt/eins/releases/9cb80cf1d329-bundle-909a6b5e9957` to
`/opt/eins/current` and rebuild; it uses the same server package, so only
`eins-web` restarts. Releases from `97173295ec12…` and earlier use the pre-split
packaging, and returning to one restarts `eins`. The original release is at
`/opt/eins/current.before-ebcf900`.

Browser tabs opened before a website update still reference the previous asset
hashes, which then return 404; reload them. No authenticated browser check of
the current assets has been made: agent threads cannot sign in through Access. A direct-IP probe from an independent external network is also
still needed; the orb's outbound proxy makes that test inconclusive. All origin
listeners were verified as loopback-only and the firewall unchanged.

An explicit Cloudflare cache-bypass rule for this hostname's `/api/*` has **not**
been added: the deployment credential lacks ruleset permissions. Nginx sends
`Cache-Control: no-store`; the default cache behavior excludes these API paths.
Before introducing custom caching, add a bypass rule matching
`http.host eq "eins.angr1st.com" and starts_with(http.request.uri.path, "/api/")`.
The authenticated edge cache behavior has not yet been checked.

## Build a release

Use an x86_64 Linux build machine (such as an orb), the Rust version in
`mise.toml`, Trunk 0.22.1, and the `wasm32-unknown-unknown` target. The
[web README](../eins_web/README.md) describes tool installation. From the root:

```sh copy Copy Build Command
mise exec -- bash deployment/build.sh
```

This creates ignored `deployment/bundle/` containing the native executable and
the compiled website. It does not contact the VPS. Trunk may download
wasm-bindgen and wasm-opt; this is why the build runs outside Nix's sandbox.
Builds should use the reviewed source revision; keep its revision and bundle
together when staging a release. Build on the same architecture as the VPS.

## First installation (after deployment approval)

The shared VPS is `root@187.126.31.101`, running NixOS 26.05. Preserve
`/etc/nixos/configuration.nix`, including Hostinger's boot/network/SSH settings
and the imports for `/opt/grocer/source/deployment/nixos.nix` and
`/opt/square/current/deployment/nixos.nix`, plus the existing Mellow module.
Do not replace that file with an Eins-only configuration.

1. Coordinate with other deployments: `nixos-rebuild switch` activates the
   **whole host**, not just Eins. Check for other rebuilds and staged app changes.
2. Transfer `deployment/nixos.nix`, `deployment/package.nix`,
   `deployment/web.nix`, and the generated
   `deployment/bundle/` into `/opt/eins/releases/<revision>/deployment/` using
   pinned-host-key SSH. Never disable SSH host-key checking. Credentials from
   other Amp projects are not automatically available in this project.
3. Build the staged package on the VPS before activation:

   ```sh copy Copy Package Build
   nix-build --no-out-link -E 'with import <nixpkgs> {}; callPackage /opt/eins/releases/REVISION/deployment/package.nix {}'
   nix-build --no-out-link -E 'with import <nixpkgs> {}; callPackage /opt/eins/releases/REVISION/deployment/web.nix {}'
   ```

   Replace `REVISION` with the staged revision. Keep the resulting Nix store paths
   and source revision for rollback.
4. Install the staged release at `/opt/eins/current` as a **real directory**.
   Preserve any previous release separately. Add
   `/opt/eins/current/deployment/nixos.nix` to the host's existing `imports` list.
   Back up the current host configuration before editing it.
5. Configure a **separate** remotely managed Cloudflared tunnel, named
   `eins-hostinger`. Do not modify the Grocer or Square tunnels. Store only its
   connector token at `/etc/eins/tunnel-token` (root-owned, mode 0600; parent
   directory mode 0700). Keep it outside Git and the world-readable Nix store.
   systemd passes it to cloudflared through `LoadCredential`; do not put its
   value in Nix configuration, command arguments, or chat.
6. Configure the approved hostname `eins.angr1st.com`:
   - Published route: all paths → **`http://127.0.0.1:8082`**.
   - Final ingress rule: **`http_status:404`**.
   - DNS: proxied CNAME to the new tunnel's `<UUID>.cfargotunnel.com`.
   - Require HTTPS at Cloudflare. Bypass caching for `/api/*`; do not apply
     Cache Everything or response buffering to the SSE endpoint.
   - Do not create an A/AAAA record pointing the hostname to the VPS.
7. With approval to activate the shared host:

   ```sh copy Copy Activation Commands
   nixos-rebuild build
   nixos-rebuild switch
   systemctl is-active eins eins-web eins-tunnel grocer grocer-tunnel square square-tunnel
   curl --fail http://127.0.0.1:8082/api/health
   ```

The health response must be `{"status":"Ready"}`. Retain the existing
firewall: no inbound 80/443/8080/8082/20243 rules are needed. Cloudflared needs
outbound DNS and TCP/UDP 7844; the existing tunnels already use that transport.
Keep SSH and Hostinger console access intact.

## Player access

The current deployment uses the restricted policy described above. Keep it
unless the owner explicitly authorizes a policy change.

**Tunnel-only origin access is not private player access.** Without Cloudflare
Access, anyone with the public hostname can register and play, but cannot bypass
Cloudflare by connecting directly to the VPS IP.

For approved players only, create a separate Cloudflare Access application and
explicit allow policy **before publishing the hostname**. Enable origin Access
JWT validation in this tunnel route using the team's name and the Eins
application's audience. Do not reuse another application's audience or change
Grocer/Square policies. Browser players log in through Access; the terminal
client does not implement Access login. Without Access, it can connect using
`eins_client https://eins.angr1st.com/api`.

## Verification and updates

- Check `ss -ltnp`: Eins listeners must be on `127.0.0.1`, not `0.0.0.0` or
  `[::]`. Verify direct-IP access fails from another machine on both public IPv4
  and IPv6, including HTTPS with the Eins hostname/SNI directed to the VPS IP.
- Check the HTTPS site's `/api/health`, `.wasm` MIME type (`application/wasm`),
  `/manifest.webmanifest` MIME type (`application/manifest+json`), and
  two-player gameplay. SSE at `/api/events` must update the other player
  without reloads. Nginx forwards the `code` header and strips `/api/`.
- Verify Grocer, Square, and Mellow remain healthy after every whole-system switch.
- In a maintenance window, stopping **only** `eins-tunnel` should make uncached
  public API requests fail while loopback health remains good; then restart it.
- Stage/build each update before replacing `/opt/eins/current`, preserve the
  previous directory, coordinate, and repeat the build/switch/health checks.
  If activation fails, restore the previous Eins directory/configuration and
  rebuild. Do not blindly run a whole-system rollback: it can undo another
  app's deployment. Never restore Grocer/Square data as part of an Eins rollback.

### Website-only updates

When only `eins_web` changed, keep the deployed `bundle/eins_server` and replace
only `bundle/public/`. Before switching, build with `nixos-rebuild build` and
compare the result with `/run/current-system`: `eins.service` must be
byte-identical and only `eins-web.service` may change. The switch then restarts
only Nginx; open SSE connections drop briefly and the browser reconnects.

Production adopted this directory's server-only `package.nix` in generation 14,
so website-only updates no longer change the server's store path. Changing the
server binary still restarts `eins`; schedule that like a server update.

**Restarting Eins ends every game and player session.** State is in memory;
there is no game database or persistent volume. NixOS rollback does not recover
games. Schedule updates when nobody is playing. Tunnel restarts disconnect SSE;
the browser reconnects automatically without requiring an API restart.
