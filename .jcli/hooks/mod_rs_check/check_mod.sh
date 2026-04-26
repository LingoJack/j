#!/usr/bin/env bash
# post_tool_execution hook: Write/Edit 后扫描目录及祖先，发现 mod.rs 提醒换成 name.rs + name/ 模式
set -euo pipefail

export HOOK_INPUT="$(cat)"

python3 /dev/stdin <<'PYEOF'
import sys, json, os

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
if not abs_path.endswith(".rs"):
    print("{}"); sys.exit(0)

# 从文件所在目录向上扫描，最多 4 层，查找 mod.rs
dir_path = os.path.dirname(abs_path)
reminders = []

for _ in range(5):
    mod_file = os.path.join(dir_path, "mod.rs")
    if os.path.isfile(mod_file):
        dir_name = os.path.basename(dir_path)
        parent_dir = os.path.dirname(dir_path)
        new_style = os.path.join(parent_dir, dir_name + ".rs")
        reminders.append(
            f"  - {mod_file} → 应改为 {new_style} + {dir_name}/"
        )
    parent_dir = os.path.dirname(dir_path)
    if parent_dir == dir_path:
        break
    dir_path = parent_dir

if reminders:
    msg = "发现 mod.rs，建议改为 name.rs + name/ 模式：\n" + "\n".join(reminders)
    print(json.dumps({"system_message": msg}, ensure_ascii=False))
else:
    print("{}")
PYEOF
