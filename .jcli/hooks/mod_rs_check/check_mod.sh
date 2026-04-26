#!/usr/bin/env bash
# post_tool_execution hook: Write/Edit 后检查 mod.rs 是否需要补充 mod 声明
set -euo pipefail

export HOOK_INPUT="$(cat)"

python3 /dev/stdin <<'PYEOF'
import sys, json, os, re

try:
    data = json.loads(os.environ.get("HOOK_INPUT", "{}"))
except:
    print("{}"); sys.exit(0)

tool_name = data.get("tool_name", "")
if tool_name not in ("Write", "Edit"):
    print("{}"); sys.exit(0)

# 提取 file_path
args = data.get("tool_arguments", "")
try:
    a = json.loads(args) if isinstance(args, str) else args
    file_path = a.get("file_path", "")
except:
    file_path = ""

if not file_path:
    print("{}"); sys.exit(0)

abs_path = os.path.abspath(os.path.expanduser(file_path))
filename = os.path.basename(abs_path)

# 跳过 mod.rs 自身和 main.rs
if filename in ("mod.rs", "main.rs"):
    print("{}"); sys.exit(0)

stem = filename[:-3] if filename.endswith(".rs") else filename
dir_path = os.path.dirname(abs_path)

reminders = []

# 1. 同目录 mod.rs → 检查 mod stem;
mod_file = os.path.join(dir_path, "mod.rs")
if os.path.isfile(mod_file):
    try:
        content = open(mod_file).read()
        if not re.search(rf'\bmod\s+{re.escape(stem)}\b', content):
            reminders.append(f"  - {mod_file} 缺少 'mod {stem};'")
    except:
        pass

# 2. 父目录 mod.rs → 当前目录是子模块时，检查父 mod.rs 是否声明了子目录
parent_dir = os.path.dirname(dir_path)
dir_name = os.path.basename(dir_path)
parent_mod = os.path.join(parent_dir, "mod.rs")
if os.path.isfile(parent_mod):
    try:
        content = open(parent_mod).read()
        if not re.search(rf'\bmod\s+{re.escape(dir_name)}\b', content):
            reminders.append(f"  - {parent_mod} 缺少 'mod {dir_name};'")
    except:
        pass

if reminders:
    msg = f"mod.rs 检查：{abs_path} 可能需要在 mod.rs 中声明：\n" + "\n".join(reminders)
    print(json.dumps({"system_message": msg}, ensure_ascii=False))
else:
    print("{}")
PYEOF
