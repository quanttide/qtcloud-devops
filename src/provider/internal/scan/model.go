// Package scan 提供子模块扫描（scan）的 HTTP API：
// 运行 src/cli 的 `code status` 命令并解析输出，让 Web 端消费 CLI 能力。
package scan

// SyncStatus 是子模块同步状态（对齐 CLI `SyncStatus` 四档）。
type SyncStatus string

const (
	StatusSynced      SyncStatus = "synced"
	StatusPendingPush SyncStatus = "pending_push"
	StatusPendingPull SyncStatus = "pending_pull"
	StatusConflict    SyncStatus = "conflict"
)

// ComponentStatus 是单个子模块的状态（对齐 CLI `ComponentStatus`）。
type ComponentStatus struct {
	Name   string     `json:"name"`
	Status SyncStatus `json:"status"`
	Ahead  int        `json:"ahead"`
	Behind int        `json:"behind"`
}

// Report 是一次扫描的完整报告（对齐 CLI `StatusReport`）。
type Report struct {
	Root       string            `json:"root"`
	Total      int               `json:"total"`
	Synced     int               `json:"synced"`
	Pending    int               `json:"pending"`
	Components []ComponentStatus `json:"components"`
}
