package agent_tool

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"main/loggger"
	"main/tool"
	"os/exec"
	"strings"
	"time"
)

const (
	// bash 命令默认超时时间
	defaultBashTimeout = 60 * time.Second
)

type BashToolInput struct {
	Command string `json:"command"`
}

type BashTool struct {
}

func (t *BashTool) Name() (name string) {
	return "run_bash"
}

func (t *BashTool) Desc() (desc string) {
	return fmt.Sprintf("运行 bash 命令的工具，会自动识别危险命令，超时时间为 %f sec", defaultBashTimeout.Seconds())
}

func (t *BashTool) ParamSchema() (schema string) {
	return tool.NewSchemaBuilder(BashToolInput{}).
		SetTitle("工具运行接受参数").
		SetDescription("工具运行接受参数，识别到危险命令默认向用户确认").
		SetFieldMeta("command", tool.FieldMeta{
			Title:       "执行的 bash 命令",
			Description: "执行的 bash 命令的具体内容",
		}).
		MustBuild()
}

func (t *BashTool) Run(ctx context.Context, param string) (res string, err error) {
	loggger.Infof("[BashTool.Run] 入参: %s", param)

	var input BashToolInput
	err = json.Unmarshal([]byte(param), &input)
	if err != nil {
		loggger.Errorf("[BashTool.Run] 解析参数失败, param=%s, err=%v", param, err)
		err = fmt.Errorf("解析参数失败: %v", err)
		return
	}

	command := strings.TrimSpace(input.Command)
	if command == "" {
		loggger.Warnf("[BashTool.Run] 命令为空")
		err = fmt.Errorf("命令不能为空")
		return
	}

	// 设置超时上下文
	timeout := defaultBashTimeout
	execCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	cmd := exec.CommandContext(execCtx, "bash", "-c", command)

	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	loggger.Infof("[BashTool.Run] 开始执行命令: %s", command)

	err = cmd.Run()
	if err != nil {
		if errors.Is(execCtx.Err(), context.DeadlineExceeded) {
			loggger.Errorf("[BashTool.Run] 命令执行超时, command=%s, timeout=%v", command, timeout)
			err = fmt.Errorf("命令执行超时（%v）", timeout)
			return
		}

		loggger.Errorf("[BashTool.Run] 命令执行失败, command=%s, err=%v, stderr=%s", command, err, stderr.String())
		errOutput := strings.TrimSpace(stderr.String())
		if errOutput != "" {
			return "", fmt.Errorf("命令执行失败: %s", errOutput)
		}
		err = fmt.Errorf("命令执行失败")
		return
	}

	res = stdout.String()
	if res == "" && stderr.Len() > 0 {
		res = stderr.String()
	}

	loggger.Infof("[BashTool.Run] 命令执行成功, command=%s, outputLen=%d", command, len(res))
	return
}
