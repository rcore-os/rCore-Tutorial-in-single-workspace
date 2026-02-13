#!/bin/bash

# 定义需要按顺序执行的子目录列表
DIRS=(
    "tg-sbi"
    "tg-console"
    "tg-linker"
    "tg-kernel-context"
    "tg-kernel-alloc"
    "tg-kernel-vm"
    "tg-easy-fs"
    "tg-signal-defs"
    "tg-task-manage"
    "tg-syscall"
    "tg-signal"
    "tg-signal-impl"
    "tg-sync"
    "tg-user"
    "tg-checker"
    "ch1"
    "ch1-lab"
    "ch2"
    "ch3"
    "ch4"
    "ch5"
    "ch6"
    "ch7"
    "ch8"
)

# 遍历目录列表，按顺序执行命令
for dir in "${DIRS[@]}"; do
    # 检查目录是否存在
    if [ ! -d "$dir" ]; then
        echo -e "\033[31m[ERROR] 目录 $dir 不存在，跳过执行\033[0m"
        continue
    fi

    # 进入目标目录
    echo -e "\033[32m[INFO] 进入目录：$dir\033[0m"
    cd "$dir" || {
        echo -e "\033[31m[ERROR] 无法进入目录 $dir，跳过执行\033[0m"
        continue
    }

    # 执行cargo publish命令
    echo -e "\033[33m[INFO] 在 $dir 中执行发布命令...\033[0m"
    cargo publish --dry-run --allow-dirty && cargo publish --allow-dirty
    
    # 捕获命令执行结果
    if [ $? -eq 0 ]; then
        echo -e "\033[32m[SUCCESS] $dir 发布命令执行完成\033[0m"
    else
        echo -e "\033[31m[FAIL] $dir 发布命令执行失败\033[0m"
    fi

    # 回到上级目录（当前目录）
    cd .. || {
        echo -e "\033[31m[ERROR] 无法回到上级目录，脚本终止\033[0m"
        exit 1
    }

    # 分隔线，方便查看日志
    echo "--------------------------------------------------------"
done

echo -e "\033[32m[INFO] 所有目录的命令执行完成\033[0m"
