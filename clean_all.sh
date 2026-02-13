#!/bin/bash

# 定义第一步需要执行cargo clean的子目录列表（按指定顺序）
CLEAN_DIRS=(
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

# 定义需要删除tg-user的目录列表（ch2~ch8）
RM_TGUSER_DIRS=(
    "ch2"
    "ch3"
    "ch4"
    "ch5"
    "ch6"
    "ch7"
    "ch8"
)

# 函数：打印彩色日志
print_info() {
    echo -e "\033[32m[INFO] $1\033[0m"
}

print_error() {
    echo -e "\033[31m[ERROR] $1\033[0m"
}

print_success() {
    echo -e "\033[33m[SUCCESS] $1\033[0m"
}

# 第一步：按顺序进入指定目录执行cargo clean
print_info "开始按顺序在子目录执行 cargo clean..."
for dir in "${CLEAN_DIRS[@]}"; do
    # 检查目录是否存在
    if [ ! -d "$dir" ]; then
        print_error "目录 $dir 不存在，跳过执行"
        continue
    fi

    print_info "进入目录：$dir"
    cd "$dir" || {
        print_error "无法进入目录 $dir，跳过执行"
        continue
    }

    # 执行cargo clean
    print_info "在 $dir 中执行 cargo clean..."
    cargo clean
    
    # 检查命令执行结果
    if [ $? -eq 0 ]; then
        print_success "$dir 执行 cargo clean 完成"
    else
        print_error "$dir 执行 cargo clean 失败"
    fi

    # 回到当前目录
    cd .. || {
        print_error "无法回到上级目录，脚本终止"
        exit 1
    }
    echo "--------------------------------------------------------"
done

# 第二步：在当前目录执行 cargo clean
print_info "在当前目录执行 cargo clean..."
cargo clean
if [ $? -eq 0 ]; then
    print_success "当前目录 cargo clean 完成"
else
    print_error "当前目录 cargo clean 失败"
fi
echo "--------------------------------------------------------"

# 第三步：在ch2~ch8目录下删除tg-user目录
print_info "开始在 ch2~ch8 目录删除 tg-user 目录..."
for dir in "${RM_TGUSER_DIRS[@]}"; do
    # 检查目录是否存在
    if [ ! -d "$dir" ]; then
        print_error "目录 $dir 不存在，跳过删除 tg-user"
        continue
    fi

    tg_user_path="$dir/tg-user"
    if [ -d "$tg_user_path" ]; then
        print_info "删除 $tg_user_path 目录..."
        rm -rf "$tg_user_path"
        if [ $? -eq 0 ]; then
            print_success "$tg_user_path 删除完成"
        else
            print_error "$tg_user_path 删除失败"
        fi
    else
        print_info "$tg_user_path 目录不存在，无需删除"
    fi
    echo "--------------------------------------------------------"
done

# 执行完成
print_success "所有清理操作执行完毕！"
