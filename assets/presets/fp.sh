#!/bin/bash

if [ -z "$1" ]; then
    echo "用法: j fp <文件名>"
    exit 1
fi

target="$1"
results=$(find "$(pwd)" -maxdepth 3 -name "$target" -type f 2>/dev/null)

if [ -z "$results" ]; then
    echo "未找到文件: $target"
    exit 1
fi

echo "$results"