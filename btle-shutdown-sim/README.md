# BTLE Shutdown Mesh (Testground + Gossipsub)

This folder is intentionally trimmed to one authoritative demo path: a real libp2p Gossipsub experiment in Testground with BTLE-like network constraints.

## Scope

- Runtime: Testground plan in Go
- Pubsub: libp2p Gossipsub
- Objective: show block-header propagation across a hyper-local mesh during internet shutdown conditions

## Files

- Manifest: `testground/manifest.toml`
- Composition: `testground/compositions/shutdown-mesh.toml`
- Plan implementation: `testground/plans/shutdown-mesh/main.go`

## Scenario Workflow

1. Start `N=50` libp2p nodes and join a shared Gossipsub topic.
2. Apply BTLE-like `tc` shaping per node:
   - bandwidth: `1 Mbps`
   - delay: `75ms` with `25ms` jitter
   - packet loss: `3%`
   - MTU: `256`
3. Trigger shutdown behavior by dropping non-local connections.
4. Let bridge nodes (default `10%`) inject a block-header payload.
5. Measure receive latency and hop-oriented propagation characteristics.

## Run

From this directory:

```bash
cd testground
testground daemon
testground run composition -f compositions/shutdown-mesh.toml
```

## Metrics to Report in the Blog

- End-to-end propagation latency (`p50`, `p95`, max)
- Latency vs hop count
- Delivery ratio under shutdown constraints
- Churn sensitivity (when peers intermittently disconnect)
- Gossipsub control overhead behavior on constrained links
