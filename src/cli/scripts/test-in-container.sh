#!/bin/bash
# 在 Docker 容器中运行 Rust 测试和覆盖率。
# 容器隔离编译环境，崩溃不影响宿主机。
#
# 用法:
#   ./scripts/test-in-container.sh           # 跑全部测试 + 覆盖率
#   ./scripts/test-in-container.sh -- --test-threads=1  # 传参给 cargo test
#
# 依赖:
#   - Docker
#   - 项目根目录下的 Dockerfile

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_TAG="qtcloud-test-runner"

# 解析 cargo test 额外参数（-- 之后的部分）
EXTRA_ARGS=()
for arg in "$@"; do
    EXTRA_ARGS+=("$arg")
done

echo "📦 构建测试容器镜像..."
docker build \
    -f "$PROJECT_DIR/Dockerfile" \
    -t "$IMAGE_TAG" \
    -q \
    "$PROJECT_DIR"

echo "🧪 运行测试（容器内）..."
docker run --rm \
    -v "$PROJECT_DIR:/app" \
    -w /app \
    "$IMAGE_TAG" \
    sh -c "cargo test ${EXTRA_ARGS[*]:+${EXTRA_ARGS[*]}} 2>&1 && \
           cargo llvm-cov --lcov --output-path target/coverage/lcov.info 2>&1"

echo "✅ 测试全部通过，覆盖率已生成"
echo "📊 覆盖率报告: target/coverage/lcov.info"
