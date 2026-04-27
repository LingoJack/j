#!/usr/bin/env bash
# =============================================================================
# j-cli 代码合规性检查脚本
# 用法: bash scripts/check_lint.sh [--fix]  或  make check-lint
#   --fix  自动执行 cargo fmt（默认仅报告）
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_DIR="$PROJECT_ROOT/src"

# ── 阈值配置 ──────────────────────────────────────────────────────────────────
MAX_FILE_LINES=600          # 单文件超过此值 WARN
HARD_MAX_FILE_LINES=1000    # 单文件超过此值 FAIL
MAX_FUNCTION_LINES=80       # 单函数超过此值 WARN
MAX_FUNCTION_PARAMS=4       # 函数参数超过此值 WARN

# ── 颜色 ──────────────────────────────────────────────────────────────────────
C_PASS='\033[32m'; C_WARN='\033[33m'; C_FAIL='\033[31m'
C_INFO='\033[36m'; C_BOLD='\033[1m';  C_DIM='\033[2m'; C_RST='\033[0m'

DO_FIX=false
[[ "${1:-}" == "--fix" ]] && DO_FIX=true

# ── 计数器 ────────────────────────────────────────────────────────────────────
N_TOTAL=0; N_PASS=0; N_WARN=0; N_FAIL=0

pass()  { ((N_PASS++))  || true; ((N_TOTAL++)) || true; printf "  ${C_PASS}PASS${C_RST} %s\n" "$*"; }
warn()  { ((N_WARN++))  || true; ((N_TOTAL++)) || true; printf "  ${C_WARN}WARN${C_RST} %s\n" "$*"; }
fail()  { ((N_FAIL++))  || true; ((N_TOTAL++)) || true; printf "  ${C_FAIL}FAIL${C_RST} %s\n" "$*"; }
info()  { printf "  ${C_INFO}INFO${C_RST} %s\n" "$*"; }
hdr()   { printf "\n${C_BOLD}%s${C_RST}\n" "$*"; }
dim()   { printf "      ${C_DIM}%s${C_RST}\n" "$*"; }

# ── 辅助: 查找全部 .rs 源文件 ─────────────────────────────────────────────────
all_rs() { find "$SRC_DIR" -name '*.rs' -not -path '*/target/*'; }

# =============================================================================
# 1. cargo fmt
# =============================================================================
hdr "=== 1. 代码格式 (cargo fmt) ==="
if $DO_FIX; then
    cargo fmt
    pass "cargo fmt -- 已自动格式化"
else
    if cargo fmt -- --check 2>/dev/null; then
        pass "cargo fmt 检查通过"
    else
        fail "cargo fmt 未通过，运行 'cargo fmt' 或 'bash tools/check_lint.sh --fix'"
    fi
fi

# =============================================================================
# 2. cargo clippy
# =============================================================================
hdr "=== 2. Clippy 静态分析 (-D warnings) ==="
if cargo clippy -- -D warnings 2>&1; then
    pass "clippy 零告警"
else
    fail "clippy 存在告警，详见上方输出"
fi

# =============================================================================
# 3. 单文件行数
# =============================================================================
hdr "=== 3. 单文件行数 (WARN > $MAX_FILE_LINES | FAIL >= $HARD_MAX_FILE_LINES) ==="
oversized=0
while IFS= read -r f; do
    lines=$(wc -l < "$f")
    rel="${f#$PROJECT_ROOT/}"
    if (( lines >= HARD_MAX_FILE_LINES )); then
        fail "$rel — ${lines} 行 (>= ${HARD_MAX_FILE_LINES})"
        ((oversized++)) || true
    elif (( lines > MAX_FILE_LINES )); then
        warn "$rel — ${lines} 行 (> ${MAX_FILE_LINES})"
        ((oversized++)) || true
    fi
done < <(all_rs)
if (( oversized == 0 )); then
    pass "所有文件 <= ${MAX_FILE_LINES} 行"
fi

# =============================================================================
# 4. 单函数行数
# =============================================================================
hdr "=== 4. 单函数行数 (> $MAX_FUNCTION_LINES 行 → WARN) ==="
fn_warn=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    # awk: 追踪大括号深度，在 fn 定义起点到闭合 } 之间计算行数
    result=$(awk -v file="$rel" -v max="$MAX_FUNCTION_LINES" '
    {
        # 追踪是否处于 #[cfg(test)] 模块内 — 简化: 跳过 test fn
    }
    /^[[:space:]]*(pub\s+)?(async\s+)?fn\s+[a-zA-Z_]/ && !/test/ {
        start = NR
        name = $0
        sub(/^[[:space:]]+/, "", name)
        sub(/\{.*$/, "", name)
        depth = 0; opened = 0
        do {
            line = $0
            gsub(/[^\{\}]/, "", line)
            for (i = 1; i <= length(line); i++) {
                c = substr(line, i, 1)
                if (c == "{") { depth++; opened = 1 }
                if (c == "}") { depth-- }
            }
            if (opened && depth <= 0) {
                len = NR - start + 1
                if (len > max) printf "  WARN %s — %s (%d 行)\n", file, name, len
                next
            }
        } while (getline > 0)
    }
    ' "$f")
    if [[ -n "$result" ]]; then
        echo "$result"
        ((fn_warn++)) || true
    fi
done < <(all_rs)
if (( fn_warn == 0 )); then
    pass "所有函数 <= ${MAX_FUNCTION_LINES} 行"
fi

# =============================================================================
# 5. 函数参数数量
# =============================================================================
hdr "=== 5. 函数参数数量 (> $MAX_FUNCTION_PARAMS → WARN) ==="
param_warn=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    # 用 awk 精确统计函数签名中的参数逗号数量
    result=$(awk -v file="$rel" -v max="$MAX_FUNCTION_PARAMS" '
    /^[[:space:]]*(pub\s+)?(async\s+)?fn\s+\w+/ {
        # 收集完整的函数签名（可能跨行）
        sig = $0
        while (index(sig, ")") == 0 && getline > 0) sig = sig " " $0
        # 提取括号内内容
        lparen = index(sig, "(")
        rparen = index(sig, ")")
        if (lparen > 0 && rparen > lparen) {
            params = substr(sig, lparen + 1, rparen - lparen - 1)
            gsub(/[[:space:]]+/, " ", params)
            if (length(params) == 0) next  # 无参数
            # 统计顶层逗号（忽略泛型/嵌套括号内的逗号）
            n = 1; depth = 0
            for (i = 1; i <= length(params); i++) {
                c = substr(params, i, 1)
                if (c == "<" || c == "[" || c == "(") depth++
                if (c == ">" || c == "]" || c == ")") depth--
                if (c == "," && depth == 0) n++
            }
            if (n > max) {
                line_copy = $0; sub(/^[[:space:]]+/, "", line_copy); sub(/\{.*$/, "", line_copy)
                printf "  WARN %s:%d — %s (%d 个参数)\n", file, NR, line_copy, n
            }
        }
    }
    ' "$f")
    if [[ -n "$result" ]]; then
        echo "$result"
        ((param_warn++)) || true
    fi
done < <(all_rs)
if (( param_warn == 0 )); then
    pass "所有函数参数 <= ${MAX_FUNCTION_PARAMS} 个"
fi

# =============================================================================
# 6. unwrap/expect 使用 (非 test)
# =============================================================================
hdr "=== 6. unwrap/expect 使用 (非 test 代码应避免) ==="
unwrap_warn=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    # 查找包含 .unwrap() 或 .expect( 的行，排除 test 文件
    hits=$(awk '
    /^[[:space:]]*#\[cfg\(test\)\]/ { in_test=1; next }
    /^[[:space:]]*\}/ && in_test { in_test=0 }
    !in_test && (/\.unwrap\(\)/ || /\.expect\(/) {
        # 允许 Mutex::lock().unwrap()
        if (/Mutex::lock\(\)\.unwrap\(\)/ || /RwLock::.*\.unwrap\(\)/) next
        printf "      %d: %s\n", NR, $0
    }
    ' "$f")
    if [[ -n "$hits" ]]; then
        warn "$rel:"
        echo "$hits"
        ((unwrap_warn++)) || true
    fi
done < <(all_rs)
if (( unwrap_warn == 0 )); then
    pass "非 test 代码未发现 unwrap/expect"
fi

# =============================================================================
# 7. mod.rs 检查 (严格禁止，只允许 name.rs + name/ 模式)
# =============================================================================
hdr "=== 7. mod.rs 检查 (严格禁止 — 必须使用 name.rs + name/ 模式) ==="
mod_rs_found=false
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    fail "发现 mod.rs: $rel — 必须改为 name.rs + name/ 子目录模式"
    mod_rs_found=true
done < <(find "$SRC_DIR" -name 'mod.rs')
if ! $mod_rs_found; then
    pass "未发现 mod.rs 文件，模块组织合规"
fi

# =============================================================================
# 8. super::super:: 过度层级引用检查
# =============================================================================
hdr "=== 8. super::super:: 过度层级引用 (AGENT.md 路径简化原则) ==="
super_warn=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    hits=$(grep -n 'super::super::' "$f" 2>/dev/null || true)
    if [[ -n "$hits" ]]; then
        warn "$rel — 发现 super::super:: 引用，应通过 use 导入简化:"
        echo "$hits" | sed 's/^/      /'
        ((super_warn++)) || true
    fi
done < <(all_rs)
if (( super_warn == 0 )); then
    pass "未发现 super::super:: 过度层级引用"
fi

# =============================================================================
# 9. 公共 API 文档注释
# =============================================================================
hdr "=== 9. 公共 API 文档注释 (pub fn/struct/enum/trait 需要 ///) ==="
undoc_count=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    undoc=$(awk '
    /\/\/\// { prev_doc=1; next }
    /^#\[/   { prev_attr=1; next }
    /^[[:space:]]*(pub\s+)(async\s+)?(fn|struct|enum|trait)\s+/ {
        if (!prev_doc && !prev_attr) {
            line = $0; sub(/^[[:space:]]+/, "", line)
            printf "      %d: %s\n", NR, line
        }
    }
    { prev_doc=0; prev_attr=0 }
    ' "$f")
    if [[ -n "$undoc" ]]; then
        warn "$rel — 缺少文档注释:"
        echo "$undoc"
        ((undoc_count++)) || true
    fi
done < <(all_rs)
if (( undoc_count == 0 )); then
    pass "所有公共 API 均有文档注释"
fi

# =============================================================================
# 10. TUI 输出规范
# =============================================================================
hdr "=== 10. TUI 输出规范 (禁止 println!/eprintln!/info!/error!) ==="
tui_println=0
# 检查 tui 和 chat 模块中的 println/eprintln
for subdir in tui command/chat; do
    dir="$SRC_DIR/$subdir"
    [[ -d "$dir" ]] || continue
    while IFS= read -r f; do
        rel="${f#$PROJECT_ROOT/}"
        hits=$(grep -nE '(println!|eprintln!|crate::info!|crate::error!)\(' "$f" 2>/dev/null || true)
        if [[ -n "$hits" ]]; then
            warn "$rel — 禁止在 TUI 模式使用 println/eprintln:"
            echo "$hits" | sed 's/^/      /'
            ((tui_println++)) || true
        fi
    done < <(find "$dir" -name '*.rs')
done
if (( tui_println == 0 )); then
    pass "TUI 模块未发现 println/eprintln 输出"
fi

# =============================================================================
# 11. unsafe 块 SAFETY 注释
# =============================================================================
hdr "=== 11. unsafe 块 SAFETY 注释 ==="
unsafe_warn=0
while IFS= read -r f; do
    rel="${f#$PROJECT_ROOT/}"
    hits=$(awk '
    /unsafe\s*\{/ && !/SAFETY/ {
        if (prev !~ /\/\/\s*SAFETY:/ && prev !~ /\/\*\s*SAFETY:/) {
            printf "      %d: %s\n", NR, $0
        }
    }
    { prev = $0 }
    ' "$f")
    if [[ -n "$hits" ]]; then
        warn "$rel — unsafe 块缺少 SAFETY 注释:"
        echo "$hits"
        ((unsafe_warn++)) || true
    fi
done < <(all_rs)
if (( unsafe_warn == 0 )); then
    pass "所有 unsafe 块均有 SAFETY 注释（或无 unsafe 代码）"
fi

# =============================================================================
# 汇总
# =============================================================================
hdr "=== 汇总 ==="
printf "  总检查项: ${C_BOLD}%d${C_RST}  |  ${C_PASS}PASS: %d${C_RST}  |  ${C_WARN}WARN: %d${C_RST}  |  ${C_FAIL}FAIL: %d${C_RST}\n\n" \
    "$N_TOTAL" "$N_PASS" "$N_WARN" "$N_FAIL"

if (( N_FAIL > 0 )); then
    printf "${C_FAIL}存在 FAIL 项，请修复后重新检查。${C_RST}\n"
    exit 1
elif (( N_WARN > 0 )); then
    printf "${C_WARN}存在 WARN 项，建议优化。${C_RST}\n"
    exit 0
else
    printf "${C_PASS}全部检查通过。${C_RST}\n"
    exit 0
fi
