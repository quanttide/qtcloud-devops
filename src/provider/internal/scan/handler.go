package scan

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"
)

// cliScanTimeout 限制单次 CLI 调用的最长耗时（离线扫描通常秒级返回）。
const cliScanTimeout = 60 * time.Second

// runCommand 执行一个命令，返回 stdout、stderr、退出码与启动错误。
// 仅「进程无法启动」（可执行文件缺失等）通过 err 返回；
// 已启动但非零退出时返回退出码，err 为 nil。
type runCommand func(ctx context.Context, name string, args []string) (stdout, stderr string, exitCode int, err error)

// Options 是 Handler 的构造参数；空值自动探测。
type Options struct {
	// Root 是扫描目标（含 .gitmodules 的 git 仓库根）。
	// 为空时从当前目录向上探测：最近的「git 根且含 .gitmodules」祖先目录。
	Root string
	// CLIBin 显式指定 CLI 可执行文件；为空时按
	// PATH 中的 qtcloud-devops → src/cli 预构建二进制 → cargo run 顺序探测。
	CLIBin string
	// RepoRoot 是 qtcloud-devops 仓库根（用于定位 src/cli）；
	// 为空时从当前目录向上探测 git 根。
	RepoRoot string
}

// Handler 提供 /health 与 /api/scan。
type Handler struct {
	root     string
	cliBin   string
	repoRoot string
	run      runCommand
}

// NewHandler 构造 Handler，解析默认值。
func NewHandler(opts Options) *Handler {
	cwd, err := os.Getwd()
	if err != nil {
		cwd = "."
	}
	root := opts.Root
	if root == "" {
		root = resolveScanRoot(cwd)
	}
	repoRoot := opts.RepoRoot
	if repoRoot == "" {
		repoRoot = gitRootOf(cwd)
	}
	return &Handler{
		root:     root,
		cliBin:   opts.CLIBin,
		repoRoot: repoRoot,
		run:      execCommand,
	}
}

// Routes 返回 HTTP 路由。
func (h *Handler) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", h.handleHealth)
	mux.HandleFunc("GET /api/scan", h.handleScan)
	return mux
}

// handleHealth 健康检查。
func (h *Handler) handleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// handleScan 运行 CLI scan（code status <root> --offline），返回子模块状态列表。
// CLI 不可用或调用/解析失败时返回 502 与错误信息。
func (h *Handler) handleScan(w http.ResponseWriter, r *http.Request) {
	report, err := h.scan(r.Context())
	if err != nil {
		writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, report)
}

// scan 依次尝试 CLI 候选，返回解析后的扫描报告。
func (h *Handler) scan(ctx context.Context) (*Report, error) {
	var lastErr error
	for _, c := range h.candidates() {
		stdout, stderr, exitCode, err := h.run(ctx, c.name, c.args)
		if err != nil {
			// 进程无法启动（可执行文件缺失）→ 尝试下一个候选。
			lastErr = err
			continue
		}
		if exitCode != 0 {
			msg := strings.TrimSpace(stderr)
			if msg == "" {
				msg = "无错误输出"
			}
			return nil, fmt.Errorf("qtcloud-devops code status 失败（exit %d）：%s", exitCode, msg)
		}
		report, err := ParseReport(stdout)
		if err != nil {
			return nil, err
		}
		return report, nil
	}
	return nil, fmt.Errorf("无法调用 qtcloud-devops CLI：%v", lastErr)
}

// candidate 是一个 CLI 调用候选。
type candidate struct {
	name string
	args []string
}

// candidates 返回 CLI 调用候选（对齐 studio cli_client_io.dart 的探测顺序）。
func (h *Handler) candidates() []candidate {
	if h.cliBin != "" {
		return []candidate{{name: h.cliBin, args: h.baseArgs()}}
	}
	cs := []candidate{{name: "qtcloud-devops", args: h.baseArgs()}}
	if h.repoRoot != "" {
		prebuilt := filepath.Join(h.repoRoot, "src", "cli", "target", "release", "qtcloud-devops")
		cs = append(cs, candidate{name: prebuilt, args: h.baseArgs()})
		manifest := filepath.Join(h.repoRoot, "src", "cli", "Cargo.toml")
		cs = append(cs, candidate{
			name: "cargo",
			args: append([]string{"run", "--quiet", "--manifest-path", manifest, "--"}, h.baseArgs()...),
		})
	}
	return cs
}

// baseArgs 返回 CLI scan 的实际命令：`code status <root> --offline`。
func (h *Handler) baseArgs() []string {
	return []string{"code", "status", h.root, "--offline"}
}

// execCommand 是默认命令执行实现。
func execCommand(ctx context.Context, name string, args []string) (stdout, stderr string, exitCode int, err error) {
	cmd := exec.CommandContext(ctx, name, args...)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf
	if err := cmd.Run(); err != nil {
		var ee *exec.ExitError
		if errors.As(err, &ee) {
			return outBuf.String(), errBuf.String(), ee.ExitCode(), nil
		}
		return "", "", 0, err // 进程无法启动。
	}
	return outBuf.String(), errBuf.String(), 0, nil
}

// ═══════════════════════════════════════════════════════════════════════
// 输出解析（对齐 studio cli_client.dart 的 parseStatusReport）
// ═══════════════════════════════════════════════════════════════════════

var (
	statusLabels = []string{"已同步", "待推送", "待拉取", "冲突"}
	reBothDetail = regexp.MustCompile(`\+(\d+)\s*/\s*-(\d+)`)
	reCount      = regexp.MustCompile(`\d+`)
)

// ParseReport 解析 `code status` 文本输出为 Report。
// 未知行跳过（向前兼容），无法识别的组件行忽略；缺少「组件总数」视为解析失败。
func ParseReport(output string) (*Report, error) {
	report := &Report{Components: []ComponentStatus{}}
	seenTotal := false
	for _, rawLine := range strings.Split(output, "\n") {
		line := strings.TrimSpace(rawLine)
		if line == "" {
			continue
		}
		switch {
		case strings.HasPrefix(line, "仓库:"):
			report.Root = strings.TrimSpace(strings.TrimPrefix(line, "仓库:"))
		case strings.HasPrefix(line, "组件总数:"):
			report.Total, seenTotal = parseCount(line, "组件总数:")
		case strings.HasPrefix(line, "待处理:"):
			report.Pending, _ = parseCount(line, "待处理:")
		case line == "全部组件已同步":
			// 无待处理组件，跳过。
		default:
			if c, ok := parseComponentLine(line); ok {
				report.Components = append(report.Components, c)
			}
		}
	}
	report.Synced = report.Total - report.Pending
	if !seenTotal {
		return nil, fmt.Errorf("无法解析 code status 输出：缺少「组件总数」")
	}
	return report, nil
}

// parseCount 解析形如「前缀: 数字」的行，返回数字与是否找到。
func parseCount(line, prefix string) (int, bool) {
	rest := strings.TrimSpace(strings.TrimPrefix(line, prefix))
	m := reCount.FindString(rest)
	if m == "" {
		return 0, false
	}
	n, _ := strconv.Atoi(m)
	return n, true
}

// parseComponentLine 解析组件行：`  <名称(左对齐20列)> <状态标签> <详情>`。
// 用「已知状态标签 + 前置空格」定位分隔，不依赖固定列宽——
// 名称超过 20 字符时填充消失，只剩单个空格分隔（对齐根 AGENTS.md：
// 解析外部输出用内容特征而非定界符）。
func parseComponentLine(line string) (ComponentStatus, bool) {
	label := ""
	labelIndex := -1
	for _, candidate := range statusLabels {
		idx := strings.Index(line, candidate)
		if idx > 0 && line[idx-1] == ' ' && (labelIndex < 0 || idx < labelIndex) {
			label = candidate
			labelIndex = idx
		}
	}
	if label == "" {
		return ComponentStatus{}, false
	}
	name := strings.TrimSpace(line[:labelIndex])
	detail := strings.TrimSpace(line[labelIndex+len(label):])
	ahead, behind := parseDetail(detail)
	return ComponentStatus{Name: name, Status: statusFromLabel(label), Ahead: ahead, Behind: behind}, true
}

func statusFromLabel(label string) SyncStatus {
	switch label {
	case "已同步":
		return StatusSynced
	case "待推送":
		return StatusPendingPush
	case "待拉取":
		return StatusPendingPull
	default:
		return StatusConflict
	}
}

// parseDetail 解析详情后缀，返回 (ahead, behind)。
func parseDetail(detail string) (ahead, behind int) {
	ahead = extractCount(detail, "领先")
	behind = extractCount(detail, "落后")
	if ahead > 0 || behind > 0 {
		return ahead, behind
	}
	if m := reBothDetail.FindStringSubmatch(detail); m != nil {
		ahead, _ = strconv.Atoi(m[1])
		behind, _ = strconv.Atoi(m[2])
	}
	return ahead, behind
}

func extractCount(s, marker string) int {
	idx := strings.Index(s, marker)
	if idx < 0 {
		return 0
	}
	m := reCount.FindString(s[idx+len(marker):])
	n, _ := strconv.Atoi(m)
	return n
}

// ═══════════════════════════════════════════════════════════════════════
// 扫描目标探测（对齐 studio resolveScanRoot）
// ═══════════════════════════════════════════════════════════════════════

// gitRootOf 返回 start 所属的 git 仓库根；不在任何 git 仓库中时返回 start。
func gitRootOf(start string) string {
	for dir := start; ; dir = filepath.Dir(dir) {
		if isGitRoot(dir) {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return start
		}
	}
}

func isGitRoot(dir string) bool {
	_, err := os.Stat(filepath.Join(dir, ".git"))
	return err == nil
}

// resolveScanRoot 从 start 所属的 git 根开始逐级向上，返回最近的
// 「git 仓库根且含 .gitmodules」的祖先目录（聚合仓库，如 quanttide-devops）；
// 找不到则回退到 start 所属的 git 根。
func resolveScanRoot(start string) string {
	ownRoot := gitRootOf(start)
	for dir := ownRoot; ; dir = filepath.Dir(dir) {
		if isGitRoot(dir) {
			if _, err := os.Stat(filepath.Join(dir, ".gitmodules")); err == nil {
				return dir
			}
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return ownRoot
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// 响应辅助
// ═══════════════════════════════════════════════════════════════════════

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("写响应失败: %v", err)
	}
}
