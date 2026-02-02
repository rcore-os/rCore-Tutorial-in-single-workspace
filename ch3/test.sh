#!/bin/bash
# ch3 测试脚本

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

run_base() {
    echo "运行 ch3 基础测试..."
    if cargo run 2>&1 | tg-checker --ch 3; then
        echo -e "${GREEN}✓ ch3 基础测试通过${NC}"
        return 0
    else
        echo -e "${RED}✗ ch3 基础测试失败${NC}"
        return 1
    fi
}

run_exercise() {
    echo "运行 ch3 练习测试..."
    if cargo run --features exercise 2>&1 | tg-checker --ch 3 --exercise; then
        echo -e "${GREEN}✓ ch3 练习测试通过${NC}"
        return 0
    else
        echo -e "${RED}✗ ch3 练习测试失败${NC}"
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
