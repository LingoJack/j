package main

import (
	"fmt"
	"io"
	"log"
	"os"

	markdown "github.com/MichaelMure/go-term-markdown"
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
	inputBytes, err := io.ReadAll(os.Stdin)
	if err != nil {
		fmt.Println("read from stdin failed, err:", err)
		return
	}
	content := string(inputBytes)

	width := getTerminalWidth()

	// 缩进根据宽度自适应：宽度越大，缩进越大，但保持在合理范围内
	indent := width / IndentDivisor
	if indent < MinIndent {
		indent = MinIndent
	}
	if indent > MaxIndent {
		indent = MaxIndent
	}

	result := markdown.Render(content, width, indent)
	fmt.Print(string(result))
}

// getTerminalWidth 获取终端宽度，如果无法获取则返回默认值
func getTerminalWidth() int {
	// 尝试获取终端宽度
	width, _, err := term.GetSize(int(os.Stdout.Fd()))
	if err != nil {
		// 如果获取失败（例如输出被重定向），返回默认宽度
		log.Printf("无法获取终端宽度，使用默认值%d: %v", DefaultTerminalWidth, err)
		return DefaultTerminalWidth
	}
	// 确保宽度在合理范围内
	if width < MinTerminalWidth {
		return MinTerminalWidth
	}
	if width > MaxTerminalWidth {
		return MaxTerminalWidth
	}
	return width
}
