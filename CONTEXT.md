# Glossary & Domain Terms

## Steam Datagram Relay (SDR)
Valve's private network routing protocol. Instead of connecting directly to the game server's public IP (hosted on AWS), the game client connects to a nearby Steam relay. Traffic is then routed over Steam's backhaul network to the game server.

## SDR PoP (Point of Presence)
A physical Valve relay cluster identified by a 3-letter airport code (e.g., `iad` for Sterling/Virginia, `fra` for Frankfurt). These PoPs handle the client-side connections for SDR traffic.

## Region Locking
The process of whitelisting specific matchmaking regions and blocking all others. In this project, it involves blocking both AWS IP ranges (Gamelift) and Steam SDR PoP IP ranges for the non-whitelisted regions.

## Auto-merge
A configuration directive in `nftables` interval sets. It allows the kernel to automatically merge overlapping or adjacent IP subnets, which is necessary when loading large AWS IP prefix lists that contain overlapping ranges.
