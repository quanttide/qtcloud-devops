#!/bin/bash
# 在 Docker 容器中运行 Rust 测试和覆盖率。
# 容器隔离编译环境，崩溃不影响宿主机。
#
# 用法:
#   ./scripts/test-in-container.sh             # 跑全部测试 + 覆盖率
#   ./scripts/test-in-container.sh -v           # 显示编译过程详情
#   ./scripts/test-in-container.sh -- --test-threads=1  # 传参给 cargo test
#   ./scripts/test-in-container.sh -v -- --test-threads=1  # 组合
#
# 资源限制:
#   默认 4g/2c，环境变量 TEST_MEMORY / TEST_CPUS 可覆盖
#
# 依赖:
#   - Docker
#   - 项目根目录下的 Dockerfile

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_TAG="qtcloud-test-runner"

# 资源限制（环境变量可覆盖）
MEMORY="${TEST_MEMORY:-4g}"
CPUS="${TEST_CPUS:-2}"

# ── 解析参数 ──────────────────────────────────────────────────
VERBOSE=false
CARGO_ARGS=()
PASSTHROUGH=false

for arg in "$@"; do
    if $PASSTHROUGH; then
        CARGO_ARGS+=("$arg")
    elif [ "$arg" = "--" ]; then
        PASSTHROUGH=true
    elif [ "$arg" = "-v" ] || [ "$arg" = "--verbose" ]; then
        VERBOSE=true
    else
        echo "未知参数: $arg"
        echo "用法: $0 [-v|--verbose] [-- <cargo-test-args>]"
        exit 1
    fi
done

# ── 构建镜像 ──────────────────────────────────────────────────
BUILD_OPTS=("-f" "$PROJECT_DIR/Dockerfile" "-t" "$IMAGE_TAG")
if ! $VERBOSE; then
    BUILD_OPTS+=("-q")
fi

echo "📦 构建测试容器镜像..."
docker build "${BUILD_OPTS[@]}" "$PROJECT_DIR"

# ── 容器内命令 ────────────────────────────────────────────────
CARGO_CMD="cargo test ${CARGO_ARGS[*]} 2>&1 && cargo llvm-cov --lcov --output-path target/coverage/lcov.info 2>&1"

if $VERBOSE; then
    echo "────────────────────────────────────────"
    echo "  资源限制: --memory=$MEMORY --cpus=$CPUS"
    echo "  容器命令: sh -c \"$CARGO_CMD\""
    echo "────────────────────────────────────────"
fi

echo "🧪 运行测试（容器内）..."
docker run --rm \
    --memory="$MEMORY" \
    --memory-swap="$MEMORY" \
    --cpus="$CPUS" \
    -v "$PROJECT_DIR:/app" \
    -w /app \
    "$IMAGE_TAG" \
    sh -c "$CARGO_CMD"

echo "✅ 测试全部通过，覆盖率已生成"
echo "📊 覆盖率报告: target/coverage/lcov.info"
echo "⚠ 资源限制: ${MEMORY} / ${CPUS} 核（TEST_MEMORY / TEST_CPUS 可覆盖）"
