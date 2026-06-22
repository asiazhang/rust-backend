#!/usr/bin/env bash
# ============================================================
#  scripts/dev.sh — 本地开发环境一键启动脚本
#  用途：启动依赖服务（PostgreSQL + Redis）并运行 Rust 应用，供本地调试
#  用法：bash scripts/dev.sh [命令]
#
#  容器运行时：docker (Docker Desktop / Colima 等)
# ============================================================

set -euo pipefail

# ===== 路径配置 =====
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/db_helper/docker-compose.yml"

# ===== 颜色输出 =====
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${BLUE}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
error() { printf "${RED}[ERROR]${NC} %s\n" "$*" >&2; }

# ===== 帮助 =====
usage() {
    cat <<EOF
🚀 Rust Backend 本地开发启动脚本

用法: bash scripts/dev.sh [命令]

容器运行时:
  docker   Docker Desktop / Colima 等

命令:
  up       启动依赖服务 + 数据库迁移 + 运行应用 (默认)
  deps     仅启动依赖服务 (PostgreSQL + Redis) 并执行迁移
  app      仅运行应用 (cargo run)
  migrate  仅运行数据库迁移
  down     停止依赖服务
  status   查看依赖服务状态
  logs     查看依赖服务日志 (实时跟踪)
  help     显示此帮助信息

示例:
  bash scripts/dev.sh            # 一键启动全部
  bash scripts/dev.sh deps       # 只启动依赖服务
  bash scripts/dev.sh down       # 停止依赖服务
EOF
}

# ===== 依赖检查 =====
check_deps() {
    if ! command -v docker >/dev/null 2>&1; then
        error "未找到 docker，请安装 Docker Desktop / Colima / Docker Engine"
        exit 1
    fi
    info "📦 使用容器运行时: ${CYAN}docker${NC}"
    if ! docker info >/dev/null 2>&1; then
        error "Docker 守护进程未运行，请先启动 Docker Desktop 或 colima start"
        exit 1
    fi
    if ! docker compose version >/dev/null 2>&1; then
        error "未找到 docker compose v2 插件"
        info "请安装 Docker Compose 插件: brew install docker-compose 并链接到 ~/.docker/cli-plugins/"
        exit 1
    fi
}

# ===== 等待服务就绪 =====
wait_for_postgres() {
    info "⏳ 等待 PostgreSQL 就绪..."
    local i
    for i in {1..30}; do
        if docker compose -f "$COMPOSE_FILE" exec -T postgresql pg_isready -U postgres >/dev/null 2>&1; then
            ok "✅ PostgreSQL 已就绪"
            return 0
        fi
        sleep 1
    done
    error "❌ PostgreSQL 启动超时"
    exit 1
}

wait_for_redis() {
    info "⏳ 等待 Redis 就绪..."
    local i
    for i in {1..30}; do
        # 在容器内执行，利用容器内的 REDIS_PASSWORD 环境变量，避免硬编码密码
        if docker compose -f "$COMPOSE_FILE" exec -T redis sh -c 'redis-cli -a "$REDIS_PASSWORD" --no-auth-warning ping' >/dev/null 2>&1; then
            ok "✅ Redis 已就绪"
            return 0
        fi
        sleep 1
    done
    error "❌ Redis 启动超时"
    exit 1
}

# ===== 启动依赖服务 =====
start_deps() {
    info "🐳 启动依赖服务 (PostgreSQL + Redis) via docker..."
    docker compose -f "$COMPOSE_FILE" up -d
    wait_for_postgres
    wait_for_redis
    ok "🎉 依赖服务已就绪"
    echo ""
    docker compose -f "$COMPOSE_FILE" ps
    echo ""
}

# ===== 数据库迁移 =====
run_migrate() {
    if ! command -v sqlx >/dev/null 2>&1; then
        warn "⚠️  未找到 sqlx-cli，跳过数据库迁移"
        warn "   请运行: cargo install sqlx-cli"
        return 0
    fi
    info "📦 初始化/迁移数据库..."
    (cd "$PROJECT_ROOT" && sqlx database create) || true
    (cd "$PROJECT_ROOT" && sqlx migrate run)
    ok "✅ 数据库迁移完成"
}

# ===== 启动应用 =====
start_app() {
    info "🚀 启动 Rust 应用..."
    cd "$PROJECT_ROOT"
    if cargo watch --version >/dev/null 2>&1; then
        info "♻️  检测到 cargo-watch，启用热重载模式"
        cargo watch -x run
    else
        info "💡 提示: 安装 cargo-watch 可启用热重载: cargo install cargo-watch"
        cargo run
    fi
}

# ===== 停止依赖 =====
stop_deps() {
    info "🛑 停止依赖服务..."
    docker compose -f "$COMPOSE_FILE" down
    ok "✅ 依赖服务已停止"
}

# ===== 状态 / 日志 =====
status_deps() {
    docker compose -f "$COMPOSE_FILE" ps
}

logs_deps() {
    info "📜 跟踪依赖服务日志 (Ctrl+C 退出)..."
    docker compose -f "$COMPOSE_FILE" logs -f
}

# ===== 主流程 =====
main() {
    local cmd="${1:-up}"
    case "$cmd" in
        up)
            check_deps
            start_deps
            run_migrate
            start_app
            ;;
        deps)
            check_deps
            start_deps
            run_migrate
            ;;
        app)
            start_app
            ;;
        migrate)
            run_migrate
            ;;
        down)
            stop_deps
            ;;
        status|ps)
            status_deps
            ;;
        logs)
            logs_deps
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            error "未知命令: $cmd"
            echo ""
            usage
            exit 1
            ;;
    esac
}

main "$@"
