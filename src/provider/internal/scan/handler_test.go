package scan

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════
// 输出解析（纯函数）
// ═══════════════════════════════════════════════════════════════════════

func TestParseReportFull(t *testing.T) {
	output := `仓库: /home/user/repo
组件总数: 4
待处理: 3
  libs/sub             待推送 (领先 2 提交)
  docs/a               待拉取 (落后 3 提交)
  data/x               冲突 (+1/-2)
`
	report, err := ParseReport(output)
	if err != nil {
		t.Fatalf("ParseReport 出错: %v", err)
	}
	if report.Root != "/home/user/repo" {
		t.Errorf("Root = %q, 期望 /home/user/repo", report.Root)
	}
	if report.Total != 4 || report.Pending != 3 || report.Synced != 1 {
		t.Errorf("计数 = total %d/pending %d/synced %d, 期望 4/3/1", report.Total, report.Pending, report.Synced)
	}
	if len(report.Components) != 3 {
		t.Fatalf("组件数 = %d, 期望 3", len(report.Components))
	}
	want := []ComponentStatus{
		{Name: "libs/sub", Status: StatusPendingPush, Ahead: 2, Behind: 0},
		{Name: "docs/a", Status: StatusPendingPull, Ahead: 0, Behind: 3},
		{Name: "data/x", Status: StatusConflict, Ahead: 1, Behind: 2},
	}
	for i, c := range report.Components {
		if c != want[i] {
			t.Errorf("组件[%d] = %+v, 期望 %+v", i, c, want[i])
		}
	}
}

func TestParseReportAllSynced(t *testing.T) {
	output := `仓库: /home/user/repo
组件总数: 2
全部组件已同步
`
	report, err := ParseReport(output)
	if err != nil {
		t.Fatalf("ParseReport 出错: %v", err)
	}
	if report.Pending != 0 || report.Synced != 2 || len(report.Components) != 0 {
		t.Errorf("全部同步时 = pending %d/synced %d/components %d, 期望 0/2/0",
			report.Pending, report.Synced, len(report.Components))
	}
}

func TestParseReportSkipsUnknownLine(t *testing.T) {
	output := `仓库: /r
组件总数: 1
未知新格式行，向前兼容跳过
全部组件已同步
`
	report, err := ParseReport(output)
	if err != nil {
		t.Fatalf("ParseReport 出错: %v", err)
	}
	if report.Total != 1 || len(report.Components) != 0 {
		t.Errorf("未知行应跳过: total %d/components %d", report.Total, len(report.Components))
	}
}

func TestParseReportMissingTotal(t *testing.T) {
	_, err := ParseReport("完全不是 status 输出\n")
	if err == nil {
		t.Fatal("缺少「组件总数」应报错")
	}
}

func TestParseComponentLineLongName(t *testing.T) {
	// 名称超过 20 字符时填充消失，只剩单个空格分隔。
	line := "a-very-long-component-name-that-overflows 待推送 (领先 5 提交)"
	c, ok := parseComponentLine(line)
	if !ok {
		t.Fatal("应识别组件行")
	}
	if c.Name != "a-very-long-component-name-that-overflows" || c.Status != StatusPendingPush || c.Ahead != 5 {
		t.Errorf("长名称解析 = %+v", c)
	}
}

func TestParseDetail(t *testing.T) {
	cases := []struct {
		detail        string
		ahead, behind int
	}{
		{"(领先 2 提交)", 2, 0},
		{"(落后 3 提交)", 0, 3},
		{"(+1/-2)", 1, 2},
		{"", 0, 0},
		{"(领先 1 提交) (落后 1 提交)", 1, 1},
	}
	for _, c := range cases {
		ahead, behind := parseDetail(c.detail)
		if ahead != c.ahead || behind != c.behind {
			t.Errorf("parseDetail(%q) = %d/%d, 期望 %d/%d", c.detail, ahead, behind, c.ahead, c.behind)
		}
	}
}

func TestStatusFromLabel(t *testing.T) {
	cases := map[string]SyncStatus{
		"已同步": StatusSynced,
		"待推送": StatusPendingPush,
		"待拉取": StatusPendingPull,
		"冲突":  StatusConflict,
		"未知":  StatusConflict,
	}
	for label, want := range cases {
		if got := statusFromLabel(label); got != want {
			t.Errorf("statusFromLabel(%q) = %s, 期望 %s", label, got, want)
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// 扫描目标探测
// ═══════════════════════════════════════════════════════════════════════

// fakeRepo 在 dir 下构造 git 根（含 .git 目录），可选 .gitmodules。
func fakeRepo(t *testing.T, dir string, gitmodules bool) {
	t.Helper()
	if err := os.MkdirAll(filepath.Join(dir, ".git"), 0o755); err != nil {
		t.Fatalf("创建 .git 失败: %v", err)
	}
	if gitmodules {
		if err := os.WriteFile(filepath.Join(dir, ".gitmodules"), []byte("[submodule]\n"), 0o644); err != nil {
			t.Fatalf("创建 .gitmodules 失败: %v", err)
		}
	}
}

func TestResolveScanRoot(t *testing.T) {
	base := t.TempDir()
	// base/apps/qtcloud-devops/src/provider（provider 工作目录）
	app := filepath.Join(base, "apps", "qtcloud-devops")
	provider := filepath.Join(app, "src", "provider")
	// 聚合仓库：base（git 根 + .gitmodules）
	fakeRepo(t, base, true)
	// app 是普通 git 根（无 .gitmodules，如 qtcloud-devops 应用仓库）
	fakeRepo(t, app, false)
	// provider 不是 git 根
	if err := os.MkdirAll(provider, 0o755); err != nil {
		t.Fatalf("创建 provider 目录失败: %v", err)
	}

	root := resolveScanRoot(provider)
	if root != base {
		t.Errorf("resolveScanRoot = %q, 期望聚合仓库根 %q", root, base)
	}
	// 从聚合仓库自身开始 → 直接命中。
	if root := resolveScanRoot(base); root != base {
		t.Errorf("resolveScanRoot(聚合根) = %q, 期望 %q", root, base)
	}
	// gitRootOf 停在最近的 git 根（app），不上溯。
	if root := gitRootOf(provider); root != app {
		t.Errorf("gitRootOf = %q, 期望 %q", root, app)
	}
}

func TestResolveScanRootFallback(t *testing.T) {
	base := t.TempDir()
	// 无任何 git 仓库 → 回退到传入起点（起点自身）。
	if root := resolveScanRoot(filepath.Join(base, "nowhere")); root != filepath.Join(base, "nowhere") {
		t.Errorf("无 git 根时回退 = %q, 期望起点自身", root)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Handler（注入假 runner）
// ═══════════════════════════════════════════════════════════════════════

func newTestHandler(t *testing.T, run runCommand) *Handler {
	t.Helper()
	h := NewHandler(Options{
		Root:     "/fake/scan/root",
		CLIBin:   "/fake/bin/qtcloud-devops",
		RepoRoot: "/fake/repo",
	})
	h.run = run
	return h
}

func fakeRun(stdout string, exitCode int, err error) runCommand {
	return func(ctx context.Context, name string, args []string) (string, string, int, error) {
		return stdout, "stderr-msg", exitCode, err
	}
}

func doRequest(t *testing.T, h *Handler, method, path string) *httptest.ResponseRecorder {
	t.Helper()
	req := httptest.NewRequest(method, path, nil)
	rec := httptest.NewRecorder()
	h.Routes().ServeHTTP(rec, req)
	return rec
}

func TestHealth(t *testing.T) {
	h := newTestHandler(t, fakeRun("", 0, nil))
	rec := doRequest(t, h, http.MethodGet, "/health")
	if rec.Code != http.StatusOK {
		t.Fatalf("健康检查 = %d, 期望 200", rec.Code)
	}
	var body map[string]string
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil || body["status"] != "ok" {
		t.Errorf("健康检查响应 = %q, 期望 {\"status\":\"ok\"}", rec.Body.String())
	}
}

func TestScanOK(t *testing.T) {
	output := `仓库: /fake/scan/root
组件总数: 1
待处理: 1
  libs/sub             待推送 (领先 2 提交)
`
	h := newTestHandler(t, fakeRun(output, 0, nil))
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusOK {
		t.Fatalf("/api/scan = %d, 期望 200（body: %s）", rec.Code, rec.Body.String())
	}
	var report Report
	if err := json.Unmarshal(rec.Body.Bytes(), &report); err != nil {
		t.Fatalf("响应不是合法 JSON: %v", err)
	}
	if report.Root != "/fake/scan/root" || report.Total != 1 || report.Pending != 1 || report.Synced != 0 {
		t.Errorf("报告 = %+v", report)
	}
	if len(report.Components) != 1 || report.Components[0].Name != "libs/sub" ||
		report.Components[0].Status != StatusPendingPush || report.Components[0].Ahead != 2 {
		t.Errorf("组件 = %+v", report.Components)
	}
}

func TestScanCLIUnavailable(t *testing.T) {
	// 进程无法启动（可执行文件缺失）→ 502。
	h := newTestHandler(t, fakeRun("", 0, os.ErrNotExist))
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusBadGateway {
		t.Fatalf("/api/scan = %d, 期望 502", rec.Code)
	}
	var body map[string]string
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil ||
		!strings.Contains(body["error"], "无法调用 qtcloud-devops CLI") {
		t.Errorf("502 响应 = %q, 应含 CLI 不可用信息", rec.Body.String())
	}
}

func TestScanCLINonZeroExit(t *testing.T) {
	// 命令已启动但非零退出 → 502 与错误信息。
	h := newTestHandler(t, fakeRun("", 1, nil))
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusBadGateway {
		t.Fatalf("/api/scan = %d, 期望 502", rec.Code)
	}
	if !strings.Contains(rec.Body.String(), "exit 1") || !strings.Contains(rec.Body.String(), "stderr-msg") {
		t.Errorf("502 响应 = %q, 应含退出码与 stderr", rec.Body.String())
	}
}

func TestScanParseFailure(t *testing.T) {
	// CLI 输出不可解析 → 502。
	h := newTestHandler(t, fakeRun("not a status report", 0, nil))
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusBadGateway {
		t.Fatalf("/api/scan = %d, 期望 502", rec.Code)
	}
	if !strings.Contains(rec.Body.String(), "组件总数") {
		t.Errorf("502 响应 = %q, 应含解析错误信息", rec.Body.String())
	}
}

func TestScanCallsCLIWithActualCommand(t *testing.T) {
	// 验证传给 CLI 的是实际命令：`code status <root> --offline`。
	output := `仓库: /fake/scan/root
组件总数: 0
全部组件已同步
`
	var gotName string
	var gotArgs []string
	h := NewHandler(Options{Root: "/fake/scan/root", CLIBin: "/fake/bin/qtcloud-devops"})
	h.run = func(ctx context.Context, name string, args []string) (string, string, int, error) {
		gotName, gotArgs = name, args
		return output, "", 0, nil
	}
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusOK {
		t.Fatalf("/api/scan = %d, 期望 200", rec.Code)
	}
	if gotName != "/fake/bin/qtcloud-devops" {
		t.Errorf("CLI = %q", gotName)
	}
	want := []string{"code", "status", "/fake/scan/root", "--offline"}
	if len(gotArgs) != len(want) {
		t.Fatalf("args = %v, 期望 %v", gotArgs, want)
	}
	for i := range want {
		if gotArgs[i] != want[i] {
			t.Fatalf("args = %v, 期望 %v", gotArgs, want)
		}
	}
}

func TestScanRetriesNextCandidateOnSpawnFailure(t *testing.T) {
	// 第一个候选无法启动 → 尝试下一个（PATH 二进制 → 预构建 → cargo）。
	output := `仓库: /fake/scan/root
组件总数: 0
全部组件已同步
`
	var calls []string
	h := NewHandler(Options{Root: "/fake/scan/root", RepoRoot: "/fake/repo"})
	h.run = func(ctx context.Context, name string, args []string) (string, string, int, error) {
		calls = append(calls, name)
		if name == "qtcloud-devops" {
			return "", "", 0, os.ErrNotExist // 第一个候选启动失败
		}
		return output, "", 0, nil
	}
	rec := doRequest(t, h, http.MethodGet, "/api/scan")
	if rec.Code != http.StatusOK {
		t.Fatalf("/api/scan = %d, 期望 200（body: %s）", rec.Code, rec.Body.String())
	}
	want := []string{"qtcloud-devops", "/fake/repo/src/cli/target/release/qtcloud-devops"}
	if len(calls) != len(want) {
		t.Fatalf("候选调用 = %v, 期望 %v", calls, want)
	}
	for i := range want {
		if calls[i] != want[i] {
			t.Fatalf("候选调用 = %v, 期望 %v", calls, want)
		}
	}
}

func TestMethodNotAllowed(t *testing.T) {
	h := newTestHandler(t, fakeRun("", 0, nil))
	if rec := doRequest(t, h, http.MethodPost, "/api/scan"); rec.Code != http.StatusMethodNotAllowed {
		t.Errorf("POST /api/scan = %d, 期望 405", rec.Code)
	}
}
