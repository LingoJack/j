package main

import (
	"agent_engine/constant"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"strings"

	markdown "github.com/MichaelMure/go-term-markdown"
	flag "github.com/spf13/pflag"
	"github.com/tidwall/gjson"
	"golang.org/x/term"
)

const (
	// DefaultTerminalWidth 终端宽度相关常量
	DefaultTerminalWidth = 80  // 默认终端宽度
	MinTerminalWidth     = 40  // 最小终端宽度
	MaxTerminalWidth     = 200 // 最大终端宽度
	IndentDivisor        = 20  // 缩进计算除数（宽度/20）
	MinIndent            = 2   // 最小缩进
	MaxIndent            = 8   // 最大缩进
)

func main() {
	var inputContent string
	inputBytes, err := io.ReadAll(os.Stdin)
	if err != nil {
		fmt.Println("read from stdin failed, err:", err)
		return
	}
	inputContent = string(inputBytes)

	// render 命令不需要加载配置文件，直接渲染输出
	if *command == "render" {
		// 直接使用 markdown 渲染并输出
		transport(inputContent, false)
		return
	}
}
