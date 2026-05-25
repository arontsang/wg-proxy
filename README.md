# Wireguard HTTP Proxy

## The ugly (why I am abandoning this project)

At present, I am still struggling to optimize the throughput of this setup. Currently, the setup struggles to get over 20Mbps
over a link with latency 300ms.

I suspect that the bottleneck comes from GSO/GRO optimizations. Currently we send/receive one packet at a time (a naive attempt
at GSO yielded a whooping 2 packets per syscall), due to how Tokio's lack of GSO/GRO/Sendmmsg etc. This observation is backed up
by the fact that QUIC performs similarly poorly when GSO/GRO is disabled.

In order to improve throughput on this project, I would need to either:

 - Write a Sendmmsg function, that would yield an unknown, but limited performance uplift (since GSO is faster than Sendmmsg)
 - Rewrite the entire TCP/IP stack to optimize for large blocks of packets to be written using GSO
 - Pivot to using io_uring (which Tokio does not support, which again means rewriting my TCP/IP stack to use another Runtime)

Until such time I can take on such a huge undertaking, I am pivoting to using QUIC/Stun/UDP hole punching instead (since Quinn
has already optimized the GSO pathway).

Instead I am abandoning this effort in favour for https://github.com/arontsang/iroh-vpn-proxy

## What is this is for

This is a lightweight project that is designed to be deployed to Azure's FAAS intrastructure to provide the user
with a on-demand scale to zero VPN/HTTP Proxy for use with circumventing geolocation and political firewall
restrictions, with minimal cost (scale to zero).

The project comes in two parts. A FAAS binary that is deployed to Azure's Functions. This must be configured
to dial back to the user's router (which should be running wireguard).

The second part of the project is a tiny daemon that runs on the user's network, and intercepts the HTTP proxy
requests, forwards the request to the Azure Function (over the Wireguard tunnel) whilst also calling a keep alive
endpoint on the Azure functions trigger.

## How this works

The FAAS binary contains:
 - A wireguard implementation
 - A Userland TCP/IP stack
 - A HTTP Forward Proxy

On startup, the wireguard client, phones home to the user's router, and creates a secure tunnel between the user's network
and the Azure Function. The packets are passed to the TCP/IP stack which reconstructs TCP streams, which are then passed
on to the HTTP Forward Proxy.
