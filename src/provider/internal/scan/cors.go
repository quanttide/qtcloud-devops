package scan

import (
	"net/http"
)

// corsMiddleware 支持浏览器端 Web 客户端（studio Web）跨源访问。
//
// 背景：studio Web 站点与 provider 部署域非同源，浏览器对 /api/scan
// 等请求执行 CORS 预检。两个要点（参考 qtcloud-secret handler/cors.go）：
//   - OPTIONS 预检直接 204 + CORS 头，不进入业务处理
//   - Allow-Origin 按白名单精确回显，防任意站点读取
//
// allowed 为空时不设置任何 CORS 头（同源部署或非浏览器消费端）。
func corsMiddleware(allowed []string, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		origin := r.Header.Get("Origin")
		if origin != "" && originAllowed(origin, allowed) {
			w.Header().Set("Access-Control-Allow-Origin", origin)
			w.Header().Set("Vary", "Origin")
		}

		if r.Method == http.MethodOptions {
			w.Header().Set("Access-Control-Allow-Methods", "GET, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
			w.Header().Set("Access-Control-Max-Age", "86400")
			w.WriteHeader(http.StatusNoContent)
			return
		}

		next.ServeHTTP(w, r)
	})
}

func originAllowed(origin string, allowed []string) bool {
	for _, a := range allowed {
		if origin == a {
			return true
		}
	}
	return false
}
