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
echo -e "${YELLOW}────────── cargo run 输出 ──────────${NC}"

# 使用 tee 将 cargo run 的输出同时显示在终端和传递给 tg-checker
# - cargo run 2>&1：合并 stdout 和 stderr（包含编译信息和运行输出）
# - tee /dev/stderr：将输出复制一份到 stderr（显示在终端），原始流继续通过管道
# - tg-checker --ch 2：接收管道中的输出进行检查
# 使用 pipefail 确保管道中任意命令失败都能被捕获
set -o pipefail
if cargo run 2>&1 | tee /dev/stderr | tg-checker --ch 2; then
    echo ""
    echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
    echo -e "${GREEN}✓ ch2 基础测试通过${NC}"
    exit 0
else
    echo ""
    echo -e "${YELLOW}────────── 测试结果 ──────────${NC}"
    echo -e "${RED}✗ ch2 基础测试失败${NC}"
    exit 1
fi
