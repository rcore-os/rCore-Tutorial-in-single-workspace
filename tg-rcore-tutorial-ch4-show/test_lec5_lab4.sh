#!/usr/bin/env bash
# ch4-show 验收测试：检查 lec5 知识点标签和 backtrace 输出
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "===== 构建 ch4-show（两次以获取 DWARF 数据） ====="
cargo build 2>&1 | tail -3
cargo build 2>&1 | tail -3

echo ""
echo "===== 运行 ch4-show ====="
TMPOUT="/tmp/ch4_show_test_$$.log"

ELF="target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch4-show"
timeout 60 qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel "${ELF}" \
    > "${TMPOUT}" 2>&1 || true

echo "  捕获 $(wc -l < "${TMPOUT}") 行输出"

PASS=0
FAIL=0

check_required() {
    local tag="$1"
    local desc="$2"
    if grep -qF "${tag}" "${TMPOUT}"; then
        echo "  [PASS] ${desc}"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] ${desc} -- 未找到 '${tag}'"
        FAIL=$((FAIL + 1))
    fi
}

check_optional() {
    local tag="$1"
    local desc="$2"
    if grep -qF "${tag}" "${TMPOUT}"; then
        echo "  [PASS] ${desc}"
        PASS=$((PASS + 1))
    else
        echo "  [SKIP] ${desc} -- 未找到 '${tag}'（可选）"
    fi
}

echo ""
echo "===== Lec5 知识点静态标签 ====="
check_required "[LEC5-LAB4] kp=sv39_paging"          "Sv39 分页机制"
check_required "[LEC5-LAB4] kp=address_space_isolation" "地址空间隔离"
check_required "[LEC5-LAB4] kp=identity_mapping"      "内核恒等映射"
check_required "[LEC5-LAB4] kp=kernel_heap"            "内核堆分配器"
check_required "[LEC5-LAB4] kp=multislot_portal"       "异界传送门"
check_required "[LEC5-LAB4] kp=pte_flags"              "页表项标志位"
check_required "[LEC5-LAB4] kp=multiprog"              "多道程序"
check_required "[LEC5-LAB4] kp=elf_loading"            "ELF 加载"
check_required "[LEC5-LAB4] kp=syscalls"               "系统调用表"
check_required "[LEC5-LAB4] kp=address_translation"    "地址翻译"
check_required "[LEC5-LAB4] kp=context_switch_mech"    "上下文切换机制"
check_required "[LEC5-LAB4] kp=kernel_satp"            "内核 satp"
check_required "[LEC5-LAB4] kp=privilege_levels"       "特权级"
check_required "[LEC5-LAB4] kp=compile_info"           "编译信息"
check_required "[LEC5-LAB4] kp=control_flow"           "控制流"

echo ""
echo "===== Lec5 知识点动态标签 ====="
check_required "[LEC5-LAB4] kp=kernel_space_created"   "内核地址空间创建"
check_required "[LEC5-LAB4] kp=elf_segment"            "ELF 段映射"
check_required "[LEC5-LAB4] kp=process_created"        "进程创建"
check_required "[LEC5-LAB4] kp=page_table"             "页表摘要"
check_required "[LEC5-LAB4] kp=portal_enter_user"      "传送门进入用户态"
check_required "[LEC5-LAB4] kp=syscall_trap"           "系统调用陷入"
check_required "[LEC5-LAB4] kp=process_exit"           "进程退出"
check_required "[LEC5-LAB4] kp=process_switch"         "进程切换（含 satp）"
check_optional "[LEC5-LAB4] kp=timer_interrupt"        "时钟中断（QEMU 不一定触发）"
check_optional "[LEC5-LAB4] kp=exception_kill"         "异常杀死进程"

echo ""
echo "===== Backtrace 验证 ====="
check_required "[BACKTRACE] note=fp_unwind"            "Backtrace 启动标记"
check_required "[BACKTRACE] #0"                        "Backtrace 帧 #0"
check_required "[BACKTRACE]   fn="                     "Backtrace 函数名解析"
check_required "bt_depth3"                             "正常 backtrace: bt_depth3"
check_required "bt_depth2"                             "正常 backtrace: bt_depth2"
check_required "bt_depth1"                             "正常 backtrace: bt_depth1"
check_required "rust_main"                             "正常 backtrace: rust_main"
check_required "at src/"                               "源码级行号（at src/）"
check_required "deliberate panic"                      "Panic backtrace 触发"

echo ""
echo "===== 页表内容输出 ====="
check_required "page_table_content="                   "页表内容格式化输出"
check_required "root:"                                 "页表根 PPN"

echo ""
echo "===== satp / 地址空间标签 ====="
check_required "satp="                                 "satp 值输出"
check_required "user_satp="                            "用户 satp 值"
check_required "new_satp="                             "进程切换 satp"

echo ""
echo "============================="
echo "  通过: ${PASS}"
echo "  失败: ${FAIL}"
echo "============================="

rm -f "${TMPOUT}"

if [[ ${FAIL} -gt 0 ]]; then
    echo "测试未全部通过！"
    exit 1
else
    echo "全部测试通过！"
    exit 0
fi
