# 1. Native nftables and Steam SDR blocking for region locking

## Context

Dead by Daylight matchmaking routes game client connections to specific AWS regions. In previous versions, we attempted to region-lock by updating `/etc/hosts` and then by blocking AWS IP ranges in the firewall using `iptables` and `ipset`.
However, we found that:
1. The game ignores `/etc/hosts` because it bypasses local OS DNS resolution.
2. Direct AWS IP blocks were ignored because the game routes dedicated match server connections through Steam Datagram Relay (SDR).
3. The client connects to Steam SDR PoP (Point of Presence) relays (e.g., in Stockholm or Frankfurt), and Valve routes traffic internally to the AWS servers.
4. Hence, the destination IPs seen by the OS are Steam's relay IPs, not AWS IPs.

## Decision

We will:
1. Fetch the live Steam SDR configuration from `https://api.steampowered.com/ISteamApps/GetSDRConfig/v1/?appid=381210` at runtime.
2. Define a static mapping of AWS regions (e.g., `us-east-1`) to Steam SDR PoP codes (e.g., `iad`, `atl`).
3. For any non-whitelisted region, extract the relay IPs for their corresponding Steam PoPs, convert them to `/24` subnets, and block them alongside the AWS IP ranges.
4. Use native `nftables` (with `auto-merge` interval sets and `priority filter - 10`) when available, to ensure we intercept and reject these connections before `zapret` (which runs at `priority filter` / `0`) can intercept them.
5. Fall back to `iptables`/`ipset` if `nftables` is not present.

## Consequences

- Region locking will work correctly on Linux even when SDR is active and even when `zapret` is running.
- We require fetching two remote configs (AWS IP ranges and Steam SDR config) at runtime.
- Blocking Steam SDR relay IPs might temporarily affect other Steam network features if they share the same relays, but since we only block relays of unselected regions, it does not affect local Steam connectivity.
