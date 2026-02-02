#!/bin/bash
# ch2 测试脚本

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

echo "运行 ch2 基础测试..."
if cargo run 2>&1 | tg-checker --ch 2; then
    echo -e "${GREEN}✓ ch2 基础测试通过${NC}"
    exit 0
else
    echo -e "${RED}✗ ch2 基础测试失败${NC}"
    exit 1
fi
