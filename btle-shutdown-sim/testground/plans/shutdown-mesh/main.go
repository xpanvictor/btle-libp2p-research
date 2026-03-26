package main

import (
	"context"
	"crypto/rand"
	"fmt"
	"math"
	"os/exec"
	"strconv"
	"strings"
	"time"

	libp2p "github.com/libp2p/go-libp2p"
	"github.com/libp2p/go-libp2p/core/host"
	"github.com/libp2p/go-libp2p/core/network"
	"github.com/libp2p/go-libp2p/core/peer"
	pubsub "github.com/libp2p/go-libp2p-pubsub"
	runtime "github.com/testground/sdk-go/runtime"
	"github.com/testground/sdk-go/run"
)

type config struct {
	bridgeRatio      float64
	bandwidthMbit    int
	latencyMs        int
	jitterMs         int
	lossPercent      float64
	mtu              int
	shutdownAfterMs  int
	publishAfterMs   int
	topic            string
	payloadSizeBytes int
}

func main() {
	run.Invoke(testcase)
}

func testcase(runenv *runtime.RunEnv, initCtx *run.InitContext) error {
	cfg := loadConfig(runenv)

	ctx := context.Background()
	h, err := libp2p.New()
	if err != nil {
		return err
	}
	defer h.Close()

	gossip, topic, err := setupGossipsub(ctx, h, cfg.topic)
	if err != nil {
		return err
	}
	_ = gossip

	sub, err := topic.Subscribe()
	if err != nil {
		return err
	}

	if err := applyBtleProfile(runenv, cfg); err != nil {
		return err
	}

	if err := initCtx.SyncClient.MustSignalAndWait(ctx, "ready", runenv.TestInstanceCount); err != nil {
		return err
	}

	if err := exchangePeerInfo(ctx, initCtx, h, runenv); err != nil {
		return err
	}

	if err := initCtx.SyncClient.MustSignalAndWait(ctx, "connected", runenv.TestInstanceCount); err != nil {
		return err
	}

	start := time.Now()

	if err := sleepUntil(start, cfg.shutdownAfterMs); err != nil {
		return err
	}
	applyShutdownFiltering(runenv, h)

	if err := initCtx.SyncClient.MustSignalAndWait(ctx, "shutdown", runenv.TestInstanceCount); err != nil {
		return err
	}

	injector := isBridge(runenv.TestInstanceCount, runenv.TestInstanceParams, runenv.TestInstanceCount, cfg.bridgeRatio)
	if err := sleepUntil(start, cfg.publishAfterMs); err != nil {
		return err
	}

	if injector {
		payload := makePayload(runenv, cfg.payloadSizeBytes)
		if err := topic.Publish(ctx, payload); err != nil {
			return err
		}
		runenv.RecordMessage("bridge node injected header")
		runenv.R().RecordPoint("inject_ts_ms", float64(time.Since(start).Milliseconds()))
	}

	recvCtx, cancel := context.WithTimeout(ctx, 45*time.Second)
	defer cancel()

	msg, err := sub.Next(recvCtx)
	if err != nil {
		return err
	}

	latencyMs := float64(time.Since(start).Milliseconds())
	hopGuess := estimateHopCount(msg.ReceivedFrom.String())
	runenv.R().RecordPoint("recv_latency_ms", latencyMs)
	runenv.R().RecordPoint("hop_estimate", hopGuess)
	runenv.RecordMessage(fmt.Sprintf("received header from %s in %.1fms", msg.ReceivedFrom.String(), latencyMs))

	return nil
}

func setupGossipsub(ctx context.Context, h host.Host, topicName string) (*pubsub.PubSub, *pubsub.Topic, error) {
	ps, err := pubsub.NewGossipSub(
		ctx,
		h,
		pubsub.WithMessageSigning(true),
		pubsub.WithFloodPublish(false),
		pubsub.WithPeerExchange(true),
	)
	if err != nil {
		return nil, nil, err
	}
	topic, err := ps.Join(topicName)
	if err != nil {
		return nil, nil, err
	}
	return ps, topic, nil
}

func applyBtleProfile(runenv *runtime.RunEnv, cfg config) error {
	cmds := []string{
		"tc qdisc del dev eth0 root || true",
		fmt.Sprintf("tc qdisc add dev eth0 root handle 1: tbf rate %dmbit burst 16kb latency 400ms", cfg.bandwidthMbit),
		fmt.Sprintf("tc qdisc add dev eth0 parent 1:1 handle 10: netem delay %dms %dms loss %.2f%%", cfg.latencyMs, cfg.jitterMs, cfg.lossPercent),
		fmt.Sprintf("ip link set dev eth0 mtu %d", cfg.mtu),
	}

	for _, c := range cmds {
		out, err := exec.Command("sh", "-lc", c).CombinedOutput()
		runenv.RecordMessage(fmt.Sprintf("net profile: %s -> %s", c, strings.TrimSpace(string(out))))
		if err != nil {
			return fmt.Errorf("failed command '%s': %w", c, err)
		}
	}
	return nil
}

func applyShutdownFiltering(runenv *runtime.RunEnv, h host.Host) {
	for _, conn := range h.Network().Conns() {
		if !isLocalConn(conn.RemotePeer().String()) {
			_ = conn.Close()
		}
	}
}

func isLocalConn(peerID string) bool {
	// In a real deployment this should map to geohash or zone tags.
	return strings.HasSuffix(peerID, "A") || strings.HasSuffix(peerID, "B") || strings.HasSuffix(peerID, "C")
}

func exchangePeerInfo(ctx context.Context, initCtx *run.InitContext, h host.Host, runenv *runtime.RunEnv) error {
	seq := runenv.TestSeq
	if err := initCtx.NetClient.MustPublish(ctx, "peer-info", &peer.AddrInfo{
		ID:    h.ID(),
		Addrs: h.Addrs(),
	}); err != nil {
		return err
	}

	sub, err := initCtx.NetClient.Subscribe(ctx, "peer-info", runenv.TestInstanceCount)
	if err != nil {
		return err
	}

	for i := 0; i < runenv.TestInstanceCount; i++ {
		var info peer.AddrInfo
		if err := sub.Next(ctx, &info); err != nil {
			return err
		}
		if info.ID == h.ID() {
			continue
		}
		if err := h.Connect(ctx, info); err != nil {
			runenv.RecordMessage(fmt.Sprintf("connect failed for seq %d: %v", seq, err))
		}
	}

	h.Network().Notify(&network.NotifyBundle{})
	return nil
}

func loadConfig(runenv *runtime.RunEnv) config {
	return config{
		bridgeRatio:      runenv.FloatParam("bridge_ratio"),
		bandwidthMbit:    runenv.IntParam("btle_bandwidth_mbit"),
		latencyMs:        runenv.IntParam("btle_latency_ms"),
		jitterMs:         runenv.IntParam("btle_jitter_ms"),
		lossPercent:      runenv.FloatParam("btle_loss_percent"),
		mtu:              runenv.IntParam("btle_mtu"),
		shutdownAfterMs:  runenv.IntParam("shutdown_after_ms"),
		publishAfterMs:   runenv.IntParam("publish_after_ms"),
		topic:            runenv.StringParam("topic"),
		payloadSizeBytes: runenv.IntParam("payload_size"),
	}
}

func sleepUntil(start time.Time, deadlineMs int) error {
	d := time.Duration(deadlineMs)*time.Millisecond - time.Since(start)
	if d > 0 {
		time.Sleep(d)
	}
	return nil
}

func makePayload(runenv *runtime.RunEnv, n int) []byte {
	if n < 8 {
		n = 8
	}
	buf := make([]byte, n)
	_, _ = rand.Read(buf)
	copy(buf[:8], []byte("BLKHDRv1"))
	buf[8] = byte(runenv.TestSeq % 255)
	return buf
}

func estimateHopCount(peerID string) float64 {
	last := peerID[len(peerID)-1:]
	if v, err := strconv.ParseInt(last, 16, 64); err == nil {
		return float64(v%8 + 1)
	}
	return 1
}

func isBridge(instanceCount int, _ map[string]string, testSeq int, ratio float64) bool {
	if instanceCount == 0 {
		return true
	}
	bridgeCount := int(math.Max(1, math.Round(float64(instanceCount)*ratio)))
	return testSeq < bridgeCount
}
