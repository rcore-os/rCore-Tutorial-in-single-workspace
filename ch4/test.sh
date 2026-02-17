#!/bin/bash
# ch4 测试脚本
#
# 用法：
#   ./test.sh          # 运行全部测试（等价于 ./test.sh all）
#   ./test.sh base     # 仅运行基础测试
#   ./test.sh exercise # 仅运行练习测试
#   ./test.sh all      # 运行基础 + 练习测试

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

# 检查并安装 tg-checker
ensure_tg_checker() {
    if ! command -v tg-checker &> /dev/null; then
        echo -e "${YELLOW}tg-checker 未安装，正在安装...${NC}"
        if cargo install tg-checker@0.1.0-preview.1; then
            echo -e "${GREEN}✓ tg-checker 安装成功${NC}"
        else
            echo -e "${RED}✗ tg-checker 安装失败${NC}"
            exit 1
        fi
    fi
}

ensure_tg_checker

# 使用 pipefail 确保管道中任意命令失败都能被捕获
set -o pipefail

run_base() {
    echo "运行 ch4 基础测试..."
    echo -e "${YELLOW}────────── cargo run 输出 ──────────${NC}"

    # 使用 tee 将 cargo run 的输出同时显示在终端和传递给 tg-checker
    if cargo run 2>&1 | tee /dev/stderr | tg-checker --ch 4; then
        echo ""
        echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
        echo -e "${GREEN}✓ ch4 基础测试通过${NC}"
        return 0
    else
        echo ""
        echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
        echo -e "${RED}✗ ch4 基础测试失败${NC}"
        return 1
    fi
}

run_exercise() {
    echo "运行 ch4 练习测试..."
    echo -e "${YELLOW}────────── cargo run --features exercise 输出 ──────────${NC}"

    if cargo run --features exercise 2>&1 | tee /dev/stderr | tg-checker --ch 4 --exercise; then
        echo ""
        echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
        echo -e "${GREEN}✓ ch4 练习测试通过${NC}"
        return 0
    else
        echo ""
        echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
        echo -e "${RED}✗ ch4 练习测试失败${NC}"
        return 1
    fi
}

case "${1:-all}" in
    base)
        run_base
        ;;
    exercise)
        run_exercise
        ;;
    all)
        run_base
        echo ""
        run_exercise
        ;;
    *)
        echo "用法: $0 [base|exercise|all]"
        exit 1
        ;;
esac
