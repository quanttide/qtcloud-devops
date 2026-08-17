// Command server 是 qtcloud-devops 的服务端（provider）。
//
// 职责：把 src/cli 的 scan 能力（`code status <root> --offline`）
// 暴露为 HTTP API（GET /api/scan、GET /health），供 Web 端消费 CLI 能力。
//
// 定位：CLI 能力的 HTTP 适配层（网关），不做业务逻辑（无认证/存储）。
// 结构分层参考 qtcloud-secret/src/provider（cmd/server + internal），
// 但职责不同：qtcloud-secret 的 provider 是业务服务端（认证/加密/存储），
// 本 provider 只是把 CLI 远程化，服务「无法启动进程」的消费端（Web/远程）。
//
// 环境变量：
//   - QDEV_OPS_ADDR            监听地址（默认 :8080）
//   - QDEV_OPS_ROOT            扫描目标；为空时自动探测聚合仓库根
//   - QDEV_OPS_CLI_BIN         显式 CLI 路径；为空时按候选顺序探测
//   - QDEV_OPS_ALLOWED_ORIGINS 浏览器跨源白名单（逗号分隔）；空则不设 CORS
package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/quanttide/qtcloud-devops/provider/internal/scan"
)

func main() {
	addr := os.Getenv("QDEV_OPS_ADDR")
	if addr == "" {
		addr = ":8080"
	}
	h := scan.NewHandler(scan.Options{
		Root:           os.Getenv("QDEV_OPS_ROOT"),
		CLIBin:         os.Getenv("QDEV_OPS_CLI_BIN"),
		AllowedOrigins: splitComma(os.Getenv("QDEV_OPS_ALLOWED_ORIGINS")),
	})
	srv := &http.Server{
		Addr:              addr,
		Handler:           h.Routes(),
		ReadHeaderTimeout: 5 * time.Second,
		IdleTimeout:       60 * time.Second,
	}
	log.Printf("qtcloud-devops provider 启动，监听 %s", addr)

	go func() {
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("服务退出: %v", err)
		}
	}()

	// 优雅关闭：等待 SIGINT/SIGTERM 后最多 10s 完成在途请求。
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, os.Interrupt, syscall.SIGTERM)
	<-quit
	log.Println("收到退出信号，正在关闭…")
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		log.Printf("关闭超时: %v", err)
	}
}

// splitComma 拆分逗号分隔的配置项，忽略空项。
func splitComma(s string) []string {
	if s == "" {
		return nil
	}
	parts := strings.Split(s, ",")
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		if p = strings.TrimSpace(p); p != "" {
			out = append(out, p)
		}
	}
	return out
}
