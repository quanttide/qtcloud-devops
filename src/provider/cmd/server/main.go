// Command server 是 qtcloud-devops 的服务端（provider）。
//
// 职责：把 src/cli 的 scan 能力（`code status <root> --offline`）
// 暴露为 HTTP API（GET /api/scan、GET /health），供 Web 端消费 CLI 能力。
// 结构对齐 qtcloud-secret/src/provider（cmd/server/main.go + internal/{model,handler}）。
package main

import (
	"log"
	"net/http"
	"os"

	"github.com/quanttide/qtcloud-devops/provider/internal/scan"
)

func main() {
	addr := os.Getenv("QDEV_OPS_ADDR")
	if addr == "" {
		addr = ":8080"
	}
	h := scan.NewHandler(scan.Options{
		Root:   os.Getenv("QDEV_OPS_ROOT"),
		CLIBin: os.Getenv("QDEV_OPS_CLI_BIN"),
	})
	srv := &http.Server{Addr: addr, Handler: h.Routes()}
	log.Printf("qtcloud-devops provider 启动，监听 %s", addr)
	if err := srv.ListenAndServe(); err != nil {
		log.Fatalf("服务退出: %v", err)
	}
}
